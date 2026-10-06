//! WebAssembly bindings for the extension. JSON strings cross the boundary.
//!
//! Key material reaches JavaScript in exactly one place: [`unlock`] / [`create_vault`] return the
//! vault key so the MV3 service worker can keep it in `storage.session` across restarts. Everything
//! else stays in WASM memory inside a [`VaultHandle`] and is wiped when the handle is freed.

use getrandom::SysRng;
use rand_core::UnwrapErr;
use scytale_core::snapshot::{self, SeqGuard, Snapshot, SnapshotMeta};
use scytale_core::url_match::{self, Match};
use scytale_core::{
    DeviceId, Header, Hlc, Item, ItemData, ItemId, KdfParams, Key, SecretKey, Vault, VaultId, export, generator,
    import, totp,
};
use serde::Serialize;
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

type Res<T> = Result<T, JsError>;

fn rng() -> UnwrapErr<SysRng> {
    UnwrapErr(SysRng)
}

fn err(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

fn to_json(v: &impl Serialize) -> String {
    serde_json::to_string(v).expect("serializable")
}

fn kdf(m_kib: u32, t: u32, p: u32) -> KdfParams {
    KdfParams { m_kib, t, p }
}

fn key_from(bytes: &[u8]) -> Res<Key> {
    let arr: [u8; 32] = bytes.try_into().map_err(|_| err("vault key must be 32 bytes"))?;
    Ok(Key::from_bytes(arr))
}

/// Default Argon2id parameters, so JS never hardcodes them.
#[wasm_bindgen]
pub fn default_kdf() -> String {
    let k = KdfParams::DEFAULT;
    to_json(&json!({ "m_kib": k.m_kib, "t": k.t, "p": k.p }))
}

#[wasm_bindgen]
pub fn random_id() -> String {
    DeviceId::random(&mut rng()).to_hex()
}

#[wasm_bindgen]
pub fn generate_secret_key() -> String {
    SecretKey::generate(&mut rng()).to_display().to_string()
}

/// Validates and normalizes what the user typed (case, dashes, O/0, I/L/1).
#[wasm_bindgen]
pub fn normalize_secret_key(input: &str) -> Res<String> {
    Ok(SecretKey::parse(input).map_err(err)?.to_display().to_string())
}

#[wasm_bindgen(getter_with_clone)]
pub struct Created {
    pub header: Vec<u8>,
    pub vault_key: Vec<u8>,
    pub vault_id: String,
}

#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn create_vault(
    password: &str,
    secret_key: &str,
    m_kib: u32,
    t: u32,
    p: u32,
    now_ms: f64,
    device_hex: &str,
) -> Res<Created> {
    let sk = SecretKey::parse(secret_key).map_err(err)?;
    let device = DeviceId::from_hex(device_hex).map_err(err)?;
    let written = Hlc { wall_ms: now_ms as u64, counter: 0, device };
    let (header, key) = Header::create(password, &sk, kdf(m_kib, t, p), written, &mut rng()).map_err(err)?;
    Ok(Created { header: header.encode(), vault_key: key.as_bytes().to_vec(), vault_id: header.vault_id.to_hex() })
}

/// Returns the vault key, or throws "wrong master password or Secret Key".
#[wasm_bindgen]
pub fn unlock(header: &[u8], password: &str, secret_key: &str) -> Res<Vec<u8>> {
    let header = Header::decode(header).map_err(err)?;
    let sk = SecretKey::parse(secret_key).map_err(err)?;
    Ok(header.unlock(password, &sk).map_err(err)?.as_bytes().to_vec())
}

#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn change_password(
    header: &[u8],
    vault_key: &[u8],
    new_password: &str,
    secret_key: &str,
    m_kib: u32,
    t: u32,
    p: u32,
    now_ms: f64,
    device_hex: &str,
) -> Res<Vec<u8>> {
    let header = Header::decode(header).map_err(err)?;
    let sk = SecretKey::parse(secret_key).map_err(err)?;
    let device = DeviceId::from_hex(device_hex).map_err(err)?;
    let written = Hlc { wall_ms: now_ms as u64, counter: 0, device };
    let key = key_from(vault_key)?;
    Ok(header.change_password(&key, new_password, &sk, kdf(m_kib, t, p), written, &mut rng()).map_err(err)?.encode())
}

/// `{vault_id, version, kdf: {m_kib, t, p}}` without decrypting anything.
#[wasm_bindgen]
pub fn header_info(header: &[u8]) -> Res<String> {
    let h = Header::decode(header).map_err(err)?;
    Ok(to_json(&json!({
        "vault_id": h.vault_id.to_hex(),
        "version": h.version,
        "kdf": { "m_kib": h.kdf.m_kib, "t": h.kdf.t, "p": h.kdf.p },
    })))
}

/// True if header `a` should replace header `b`.
#[wasm_bindgen]
pub fn header_supersedes(a: &[u8], b: &[u8]) -> Res<bool> {
    Ok(Header::decode(a).map_err(err)?.supersedes(&Header::decode(b).map_err(err)?))
}

// --- Generator -------------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct GenOpts {
    mode: String,
    length: usize,
    lower: bool,
    upper: bool,
    digits: bool,
    symbols: bool,
    #[serde(rename = "avoidAmbiguous")]
    avoid_ambiguous: bool,
    words: usize,
    separator: String,
    capitalize: bool,
    #[serde(rename = "includeNumber")]
    include_number: bool,
}

/// `{value, bits}` for the generator screen's options object.
#[wasm_bindgen]
pub fn generate(options_json: &str) -> Res<String> {
    let o: GenOpts = serde_json::from_str(options_json).map_err(err)?;
    let (value, bits) = if o.mode == "passphrase" {
        let opts = generator::PassphraseOptions {
            words: o.words,
            separator: o.separator,
            capitalize: o.capitalize,
            include_number: o.include_number,
        };
        (generator::passphrase(&opts, &mut rng()).map_err(err)?, generator::passphrase_entropy_bits(&opts))
    } else {
        let opts = generator::PasswordOptions {
            length: o.length,
            lower: o.lower,
            upper: o.upper,
            digits: o.digits,
            symbols: o.symbols,
            avoid_ambiguous: o.avoid_ambiguous,
        };
        (generator::password(&opts, &mut rng()).map_err(err)?, generator::password_entropy_bits(&opts))
    };
    Ok(to_json(&json!({ "value": value.as_str(), "bits": bits })))
}

// --- Import / export helpers that need no vault ------------------------------------------------

#[wasm_bindgen]
pub fn import_csv(text: &str) -> Res<String> {
    let r = import::csv(text).map_err(err)?;
    Ok(to_json(&json!({ "items": r.items, "skipped": r.skipped })))
}

#[wasm_bindgen]
pub fn import_bitwarden_json(text: &str) -> Res<String> {
    let r = import::bitwarden_json(text).map_err(err)?;
    Ok(to_json(&json!({ "items": r.items, "skipped": r.skipped })))
}

#[wasm_bindgen]
pub fn open_encrypted_export(bytes: &[u8], password: &str) -> Res<String> {
    Ok(to_json(&export::open_encrypted(bytes, password).map_err(err)?))
}

// --- The unlocked vault -----------------------------------------------------------------------

#[wasm_bindgen]
pub struct VaultHandle {
    key: Key,
    vault_id: VaultId,
    device: DeviceId,
    vault: Vault,
    guard: SeqGuard,
}

fn item_id(hex: &str) -> Res<ItemId> {
    ItemId::from_hex(hex).map_err(err)
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

#[wasm_bindgen]
impl VaultHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(vault_key: &[u8], vault_id_hex: &str, device_hex: &str) -> Res<VaultHandle> {
        Ok(VaultHandle {
            key: key_from(vault_key)?,
            vault_id: VaultId::from_hex(vault_id_hex).map_err(err)?,
            device: DeviceId::from_hex(device_hex).map_err(err)?,
            vault: Vault::new(),
            guard: SeqGuard::default(),
        })
    }

    /// Restores the rollback guard saved by [`VaultHandle::guard_json`].
    pub fn set_guard(&mut self, json: &str) -> Res<()> {
        self.guard = serde_json::from_str(json).map_err(err)?;
        Ok(())
    }

    pub fn guard_json(&self) -> String {
        to_json(&self.guard)
    }

    /// Decrypts a snapshot (ours from local storage, or another device's) and merges it in.
    /// Returns `{device, seq, device_name, changed}`.
    pub fn merge_snapshot(&mut self, bytes: &[u8]) -> Res<String> {
        let (meta, snap) = snapshot::open(&self.key, self.vault_id, bytes, &mut self.guard).map_err(err)?;
        let before = self.vault.clone();
        self.vault.merge(&snap.vault);
        Ok(to_json(&json!({
            "device": meta.device.to_hex(),
            "seq": meta.seq,
            "device_name": snap.device_name,
            "changed": before != self.vault,
        })))
    }

    /// Encrypts this device's full vault for upload / local storage.
    pub fn seal(&self, seq: f64, device_name: &str) -> Res<Vec<u8>> {
        let meta = SnapshotMeta { vault_id: self.vault_id, device: self.device, seq: seq as u64 };
        let snap = Snapshot { device_name: device_name.to_owned(), vault: self.vault.clone() };
        snapshot::seal(&self.key, meta, &snap, &mut rng()).map_err(err)
    }

    /// `[{id, title, subtitle, kind, favorite}]`, no secrets.
    pub fn list(&self) -> String {
        let out: Vec<Value> = self
            .vault
            .items()
            .map(|(id, i)| {
                json!({ "id": id.to_hex(), "title": i.title, "subtitle": subtitle(i), "kind": kind(i), "favorite": i.favorite })
            })
            .collect();
        to_json(&out)
    }

    /// The item without its secrets: `{id, item, secrets, edited_ms, edited_device, history: [last_used_ms]}`.
    pub fn view(&self, id: &str) -> Res<String> {
        let id = item_id(id)?;
        let entry = self.vault.entry(&id).filter(|e| e.item.is_some()).ok_or_else(|| err("item not found"))?;
        let item = entry.item.as_ref().expect("live");
        let (item_json, secrets) = redacted(item);
        let history: Vec<u64> = self.vault.password_history(&id).iter().map(|u| u.last_used.wall_ms).collect();
        Ok(to_json(&json!({
            "id": id.to_hex(),
            "item": item_json,
            "secrets": secrets,
            "edited_ms": entry.version.wall_ms,
            "edited_device": entry.version.device.to_hex(),
            "history": history,
        })))
    }

    /// One secret: `password`, `totp` (the setup secret), `number`, `cvv` or `pin`.
    pub fn reveal(&self, id: &str, field: &str) -> Res<String> {
        let item = self.vault.get(&item_id(id)?).ok_or_else(|| err("item not found"))?;
        let value = match (&item.data, field) {
            (ItemData::Login(l), "password") => &l.password,
            (ItemData::Login(l), "totp") => &l.totp,
            (ItemData::Card(c), "number") => &c.number,
            (ItemData::Card(c), "cvv") => &c.cvv,
            (ItemData::Card(c), "pin") => &c.pin,
            _ => return Err(err("no such secret")),
        };
        Ok(value.clone())
    }

    /// A previous password, newest first (index into `view().history`).
    pub fn reveal_history(&self, id: &str, index: usize) -> Res<String> {
        let id = item_id(id)?;
        self.vault.password_history(&id).get(index).map(|u| u.password.clone()).ok_or_else(|| err("no such entry"))
    }

    /// The whole item including secrets, for the edit form.
    pub fn draft(&self, id: &str) -> Res<String> {
        let item = self.vault.get(&item_id(id)?).ok_or_else(|| err("item not found"))?;
        Ok(to_json(item))
    }

    /// Creates (no `id`) or replaces an item. Returns its id.
    pub fn upsert(&mut self, id: Option<String>, item_json: &str, now_ms: f64) -> Res<String> {
        let item: Item = serde_json::from_str(item_json).map_err(err)?;
        let id = match id {
            Some(hex) => item_id(&hex)?,
            None => ItemId::random(&mut rng()),
        };
        self.vault.upsert(id, item, now_ms as u64, self.device);
        Ok(id.to_hex())
    }

    /// Adds many items at once (import). Returns how many were added.
    pub fn add_items(&mut self, items_json: &str, now_ms: f64) -> Res<usize> {
        let items: Vec<Item> = serde_json::from_str(items_json).map_err(err)?;
        let n = items.len();
        for item in items {
            self.vault.upsert(ItemId::random(&mut rng()), item, now_ms as u64, self.device);
        }
        Ok(n)
    }

    pub fn remove(&mut self, id: &str, now_ms: f64) -> Res<bool> {
        Ok(self.vault.delete(&item_id(id)?, now_ms as u64, self.device))
    }

    /// Logins that may be filled on `page_url`, exact-host matches first: `[{id, exact}]`.
    pub fn matches(&self, page_url: &str) -> String {
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
        to_json(&out.iter().map(|(m, id)| json!({ "id": id, "exact": *m == Match::Exact })).collect::<Vec<_>>())
    }

    /// `{username, password}` for filling, only if the login matches `page_url`.
    pub fn credentials_for(&self, id: &str, page_url: &str) -> Res<String> {
        let item = self.vault.get(&item_id(id)?).ok_or_else(|| err("item not found"))?;
        let ItemData::Login(l) = &item.data else { return Err(err("not a login")) };
        if !l.urls.iter().any(|u| url_match::match_url(u, page_url, l.exact_host).is_some()) {
            return Err(err("this login is not saved for this website"));
        }
        Ok(to_json(&json!({ "username": l.username, "password": l.password })))
    }

    /// `{code, remaining, period}` for the item's one-time code.
    pub fn totp(&self, id: &str, unix_secs: f64) -> Res<String> {
        let item = self.vault.get(&item_id(id)?).ok_or_else(|| err("item not found"))?;
        let ItemData::Login(l) = &item.data else { return Err(err("not a login")) };
        let t = totp::Totp::parse(&l.totp).map_err(err)?;
        let now = unix_secs as u64;
        Ok(to_json(&json!({ "code": t.code(now), "remaining": t.seconds_remaining(now), "period": t.period })))
    }

    /// Drops tombstones older than `max_age_ms`.
    pub fn compact(&mut self, now_ms: f64, max_age_ms: f64) {
        self.vault.compact(now_ms as u64, max_age_ms as u64);
    }

    /// `{csv, omitted}`.
    pub fn export_csv(&self) -> Res<String> {
        let (csv, omitted) = export::csv(&self.vault).map_err(err)?;
        Ok(to_json(&json!({ "csv": csv.as_str(), "omitted": omitted })))
    }

    pub fn export_encrypted(&self, password: &str, m_kib: u32, t: u32, p: u32) -> Res<Vec<u8>> {
        export::encrypted(&self.vault, password, kdf(m_kib, t, p), &mut rng()).map_err(err)
    }
}
