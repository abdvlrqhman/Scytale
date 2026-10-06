//! An unlocked vault, shaped for apps: JSON views with secrets left out unless asked for one by
//! name. Shared by the WASM bindings (extension) and the Tauri commands (desktop), so redaction and
//! matching rules exist exactly once.

use rand_core::CryptoRng;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::snapshot::{self, SeqGuard, Snapshot, SnapshotMeta};
use crate::url_match::{self, Match};
use crate::{DeviceId, Error, Item, ItemData, ItemId, KdfParams, Key, Result, Vault, VaultId, export, generator};

pub struct Session {
    key: Key,
    vault_id: VaultId,
    device: DeviceId,
    vault: Vault,
    guard: SeqGuard,
}

fn not_found() -> Error {
    Error::Invalid("item not found".into())
}

fn kind(item: &Item) -> &'static str {
    match item.data {
        ItemData::Login(_) => "login",
        ItemData::Note => "note",
        ItemData::Card(_) => "card",
        ItemData::Identity(_) => "identity",
    }
}

fn subtitle(item: &Item) -> String {
    match &item.data {
        ItemData::Login(l) if !l.username.is_empty() => l.username.clone(),
        ItemData::Login(l) => l.urls.first().cloned().unwrap_or_default(),
        ItemData::Note => "Secure note".into(),
        ItemData::Card(c) => {
            let digits: String = c.number.chars().filter(char::is_ascii_digit).collect();
            if digits.len() >= 4 { format!("Ending in {}", &digits[digits.len() - 4..]) } else { "Card".into() }
        }
        ItemData::Identity(i) => i.full_name.clone(),
    }
}

/// The item with every secret blanked, plus the names of the secrets it holds.
fn redacted(item: &Item) -> (Value, Vec<&'static str>) {
    let mut copy = item.clone();
    let mut present = Vec::new();
    match &mut copy.data {
        ItemData::Login(l) => {
            if !l.password.is_empty() {
                present.push("password");
            }
            if !l.totp.is_empty() {
                present.push("totp");
            }
            l.password.clear();
            l.totp.clear();
        }
        ItemData::Card(c) => {
            for (name, field) in [("number", &mut c.number), ("cvv", &mut c.cvv), ("pin", &mut c.pin)] {
                if !field.is_empty() {
                    present.push(name);
                }
                field.clear();
            }
        }
        ItemData::Note | ItemData::Identity(_) => {}
    }
    (serde_json::to_value(&copy).expect("serializable"), present)
}

impl Session {
    pub fn new(key: Key, vault_id: VaultId, device: DeviceId) -> Self {
        Session { key, vault_id, device, vault: Vault::new(), guard: SeqGuard::default() }
    }

    pub fn key(&self) -> &Key {
        &self.key
    }

    pub fn set_guard(&mut self, json: &str) -> Result<()> {
        self.guard = serde_json::from_str(json).map_err(|e| Error::Invalid(e.to_string()))?;
        Ok(())
    }

    pub fn guard_json(&self) -> String {
        serde_json::to_string(&self.guard).expect("serializable")
    }

    /// Decrypts a snapshot and merges it in: `{device, seq, device_name, changed}`.
    pub fn merge_snapshot(&mut self, bytes: &[u8]) -> Result<Value> {
        let (meta, snap) = snapshot::open(&self.key, self.vault_id, bytes, &mut self.guard)?;
        let before = self.vault.clone();
        self.vault.merge(&snap.vault);
        Ok(json!({
            "device": meta.device.to_hex(),
            "seq": meta.seq,
            "device_name": snap.device_name,
            "changed": before != self.vault,
        }))
    }

    pub fn seal(&self, seq: u64, device_name: &str, rng: &mut impl CryptoRng) -> Result<Vec<u8>> {
        let meta = SnapshotMeta { vault_id: self.vault_id, device: self.device, seq };
        let snap = Snapshot { device_name: device_name.to_owned(), vault: self.vault.clone() };
        snapshot::seal(&self.key, meta, &snap, rng)
    }

    /// `[{id, title, subtitle, kind, favorite}]`, no secrets.
    pub fn list(&self) -> Value {
        self.vault
            .items()
            .map(|(id, i)| {
                json!({ "id": id.to_hex(), "title": i.title, "subtitle": subtitle(i), "kind": kind(i), "favorite": i.favorite })
            })
            .collect()
    }

    /// `{id, item (secrets blanked), secrets, edited_ms, edited_device, history: [last_used_ms]}`.
    pub fn view(&self, id: &str) -> Result<Value> {
        let id = ItemId::from_hex(id)?;
        let entry = self.vault.entry(&id).filter(|e| e.item.is_some()).ok_or_else(not_found)?;
        let (item_json, secrets) = redacted(entry.item.as_ref().expect("live"));
        let history: Vec<u64> = self.vault.password_history(&id).iter().map(|u| u.last_used.wall_ms).collect();
        Ok(json!({
            "id": id.to_hex(),
            "item": item_json,
            "secrets": secrets,
            "edited_ms": entry.version.wall_ms,
            "edited_device": entry.version.device.to_hex(),
            "history": history,
        }))
    }

    /// One secret: `password`, `totp` (setup secret), `number`, `cvv` or `pin`.
    pub fn reveal(&self, id: &str, field: &str) -> Result<String> {
        let item = self.vault.get(&ItemId::from_hex(id)?).ok_or_else(not_found)?;
        let value = match (&item.data, field) {
            (ItemData::Login(l), "password") => &l.password,
            (ItemData::Login(l), "totp") => &l.totp,
            (ItemData::Card(c), "number") => &c.number,
            (ItemData::Card(c), "cvv") => &c.cvv,
            (ItemData::Card(c), "pin") => &c.pin,
            _ => return Err(Error::Invalid("no such secret".into())),
        };
        Ok(value.clone())
    }

    pub fn reveal_history(&self, id: &str, index: usize) -> Result<String> {
        let id = ItemId::from_hex(id)?;
        self.vault.password_history(&id).get(index).map(|u| u.password.clone()).ok_or_else(not_found)
    }

    /// The whole item including secrets, for the edit form.
    pub fn draft(&self, id: &str) -> Result<Value> {
        let item = self.vault.get(&ItemId::from_hex(id)?).ok_or_else(not_found)?;
        Ok(serde_json::to_value(item).expect("serializable"))
    }

    pub fn upsert(&mut self, id: Option<&str>, item: Item, now_ms: u64, rng: &mut impl CryptoRng) -> Result<String> {
        let id = match id {
            Some(hex) => ItemId::from_hex(hex)?,
            None => ItemId::random(rng),
        };
        self.vault.upsert(id, item, now_ms, self.device);
        Ok(id.to_hex())
    }

    pub fn add_items(&mut self, items: Vec<Item>, now_ms: u64, rng: &mut impl CryptoRng) -> usize {
        let n = items.len();
        for item in items {
            self.vault.upsert(ItemId::random(rng), item, now_ms, self.device);
        }
        n
    }

    pub fn remove(&mut self, id: &str, now_ms: u64) -> Result<bool> {
        Ok(self.vault.delete(&ItemId::from_hex(id)?, now_ms, self.device))
    }

    /// Logins that may be filled on `page_url`, exact-host matches first: `[{id, exact}]`.
    pub fn matches(&self, page_url: &str) -> Value {
        let mut out: Vec<(Match, String)> = self
            .vault
            .items()
            .filter_map(|(id, i)| match &i.data {
                ItemData::Login(l) => l
                    .urls
                    .iter()
                    .filter_map(|u| url_match::match_url(u, page_url, l.exact_host))
                    .max()
                    .map(|m| (m, id.to_hex())),
                _ => None,
            })
            .collect();
        out.sort_by_key(|(m, _)| std::cmp::Reverse(*m));
        out.iter().map(|(m, id)| json!({ "id": id, "exact": *m == Match::Exact })).collect()
    }

    /// `{username, password}`, only if the login is saved for `page_url`.
    pub fn credentials_for(&self, id: &str, page_url: &str) -> Result<Value> {
        let item = self.vault.get(&ItemId::from_hex(id)?).ok_or_else(not_found)?;
        let ItemData::Login(l) = &item.data else { return Err(Error::Invalid("not a login".into())) };
        if !l.urls.iter().any(|u| url_match::match_url(u, page_url, l.exact_host).is_some()) {
            return Err(Error::Invalid("this login is not saved for this website".into()));
        }
        Ok(json!({ "username": l.username, "password": l.password }))
    }

    /// `{code, remaining, period}`.
    pub fn totp(&self, id: &str, unix_secs: u64) -> Result<Value> {
        let item = self.vault.get(&ItemId::from_hex(id)?).ok_or_else(not_found)?;
        let ItemData::Login(l) = &item.data else { return Err(Error::Invalid("not a login".into())) };
        let t = crate::totp::Totp::parse(&l.totp)?;
        Ok(json!({ "code": t.code(unix_secs), "remaining": t.seconds_remaining(unix_secs), "period": t.period }))
    }

    pub fn compact(&mut self, now_ms: u64, max_age_ms: u64) {
        self.vault.compact(now_ms, max_age_ms);
    }

    /// `{csv, omitted}`.
    pub fn export_csv(&self) -> Result<Value> {
        let (csv, omitted) = export::csv(&self.vault)?;
        Ok(json!({ "csv": csv.as_str(), "omitted": omitted }))
    }

    pub fn export_encrypted(&self, password: &str, kdf: KdfParams, rng: &mut impl CryptoRng) -> Result<Vec<u8>> {
        export::encrypted(&self.vault, password, kdf, rng)
    }
}

/// Generator options as the UI sends them.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratorRequest {
    pub mode: String,
    pub length: usize,
    pub lower: bool,
    pub upper: bool,
    pub digits: bool,
    pub symbols: bool,
    pub avoid_ambiguous: bool,
    pub words: usize,
    pub separator: String,
    pub capitalize: bool,
    pub include_number: bool,
}

/// `{value, bits}`.
pub fn generate(req: GeneratorRequest, rng: &mut impl CryptoRng) -> Result<Value> {
    let (value, bits) = if req.mode == "passphrase" {
        let opts = generator::PassphraseOptions {
            words: req.words,
            separator: req.separator,
            capitalize: req.capitalize,
            include_number: req.include_number,
        };
        (generator::passphrase(&opts, rng)?, generator::passphrase_entropy_bits(&opts))
    } else {
        let opts = generator::PasswordOptions {
            length: req.length,
            lower: req.lower,
            upper: req.upper,
            digits: req.digits,
            symbols: req.symbols,
            avoid_ambiguous: req.avoid_ambiguous,
        };
        (generator::password(&opts, rng)?, generator::password_entropy_bits(&opts))
    };
    Ok(json!({ "value": value.as_str(), "bits": bits }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Login;
    use rand_chacha::ChaCha20Rng;
    use rand_core::SeedableRng;

    #[test]
    fn views_never_contain_secrets() {
        let rng = &mut ChaCha20Rng::seed_from_u64(1);
        let mut s = Session::new(Key::from_bytes([1; 32]), VaultId([2; 16]), DeviceId([3; 16]));
        let login = Login {
            username: "u".into(),
            password: "TOPSECRET".into(),
            urls: vec!["https://a.com".into()],
            totp: "JBSWY3DPEHPK3PXP".into(),
            exact_host: false,
        };
        let id = s
            .upsert(
                None,
                Item { title: "A".into(), notes: String::new(), favorite: false, data: ItemData::Login(login) },
                1,
                rng,
            )
            .unwrap();
        for v in [s.list(), s.view(&id).unwrap(), s.matches("https://a.com")] {
            let text = v.to_string();
            assert!(!text.contains("TOPSECRET") && !text.contains("JBSWY3DP"), "{text}");
        }
        assert_eq!(s.reveal(&id, "password").unwrap(), "TOPSECRET");
        assert!(s.credentials_for(&id, "https://evil.com").is_err());
    }
}
