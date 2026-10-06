//! WebAssembly bindings for the extension: thin wrappers over `scytale_core`, JSON across the
//! boundary.
//!
//! Key material reaches JavaScript in exactly one place: [`unlock`] / [`create_vault`] return the
//! vault key so the MV3 service worker can keep it in `storage.session` across restarts. Everything
//! else stays in WASM memory inside a [`VaultHandle`] and is wiped when the handle is freed.

use getrandom::SysRng;
use rand_core::UnwrapErr;
use scytale_core::session::{self, Session};
use scytale_core::{DeviceId, Header, Hlc, Item, KdfParams, Key, SecretKey, VaultId, export, import};
use serde_json::json;
use wasm_bindgen::prelude::*;

type Res<T> = Result<T, JsError>;

fn rng() -> UnwrapErr<SysRng> {
    UnwrapErr(SysRng)
}

fn err(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

fn kdf(m_kib: u32, t: u32, p: u32) -> KdfParams {
    KdfParams { m_kib, t, p }
}

fn key_from(bytes: &[u8]) -> Res<Key> {
    let arr: [u8; 32] = bytes.try_into().map_err(|_| err("vault key must be 32 bytes"))?;
    Ok(Key::from_bytes(arr))
}

fn parse<T: serde::de::DeserializeOwned>(s: &str) -> Res<T> {
    serde_json::from_str(s).map_err(err)
}

#[wasm_bindgen]
pub fn default_kdf() -> String {
    let k = KdfParams::DEFAULT;
    json!({ "m_kib": k.m_kib, "t": k.t, "p": k.p }).to_string()
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
    let kdf = json!({ "m_kib": h.kdf.m_kib, "t": h.kdf.t, "p": h.kdf.p });
    Ok(json!({ "vault_id": h.vault_id.to_hex(), "version": h.version, "kdf": kdf }).to_string())
}

#[wasm_bindgen]
pub fn header_supersedes(a: &[u8], b: &[u8]) -> Res<bool> {
    Ok(Header::decode(a).map_err(err)?.supersedes(&Header::decode(b).map_err(err)?))
}

/// `{value, bits}` for the generator screen's options object.
#[wasm_bindgen]
pub fn generate(options_json: &str) -> Res<String> {
    Ok(session::generate(parse(options_json)?, &mut rng()).map_err(err)?.to_string())
}

#[wasm_bindgen]
pub fn import_csv(text: &str) -> Res<String> {
    let r = import::csv(text).map_err(err)?;
    Ok(json!({ "items": r.items, "skipped": r.skipped }).to_string())
}

#[wasm_bindgen]
pub fn import_bitwarden_json(text: &str) -> Res<String> {
    let r = import::bitwarden_json(text).map_err(err)?;
    Ok(json!({ "items": r.items, "skipped": r.skipped }).to_string())
}

#[wasm_bindgen]
pub fn open_encrypted_export(bytes: &[u8], password: &str) -> Res<String> {
    serde_json::to_string(&export::open_encrypted(bytes, password).map_err(err)?).map_err(err)
}

/// The unlocked vault. Freeing it wipes the key and items from WASM memory.
#[wasm_bindgen]
pub struct VaultHandle(Session);

#[wasm_bindgen]
impl VaultHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(vault_key: &[u8], vault_id_hex: &str, device_hex: &str) -> Res<VaultHandle> {
        Ok(VaultHandle(Session::new(
            key_from(vault_key)?,
            VaultId::from_hex(vault_id_hex).map_err(err)?,
            DeviceId::from_hex(device_hex).map_err(err)?,
        )))
    }

    pub fn set_guard(&mut self, json: &str) -> Res<()> {
        self.0.set_guard(json).map_err(err)
    }

    pub fn guard_json(&self) -> String {
        self.0.guard_json()
    }

    /// `{device, seq, device_name, changed}`.
    pub fn merge_snapshot(&mut self, bytes: &[u8]) -> Res<String> {
        Ok(self.0.merge_snapshot(bytes).map_err(err)?.to_string())
    }

    pub fn seal(&self, seq: f64, device_name: &str) -> Res<Vec<u8>> {
        self.0.seal(seq as u64, device_name, &mut rng()).map_err(err)
    }

    pub fn list(&self) -> String {
        self.0.list().to_string()
    }

    pub fn view(&self, id: &str) -> Res<String> {
        Ok(self.0.view(id).map_err(err)?.to_string())
    }

    pub fn reveal(&self, id: &str, field: &str) -> Res<String> {
        self.0.reveal(id, field).map_err(err)
    }

    pub fn reveal_history(&self, id: &str, index: usize) -> Res<String> {
        self.0.reveal_history(id, index).map_err(err)
    }

    pub fn draft(&self, id: &str) -> Res<String> {
        Ok(self.0.draft(id).map_err(err)?.to_string())
    }

    pub fn upsert(&mut self, id: Option<String>, item_json: &str, now_ms: f64) -> Res<String> {
        let item: Item = parse(item_json)?;
        self.0.upsert(id.as_deref(), item, now_ms as u64, &mut rng()).map_err(err)
    }

    pub fn add_items(&mut self, items_json: &str, now_ms: f64) -> Res<usize> {
        Ok(self.0.add_items(parse(items_json)?, now_ms as u64, &mut rng()))
    }

    pub fn remove(&mut self, id: &str, now_ms: f64) -> Res<bool> {
        self.0.remove(id, now_ms as u64).map_err(err)
    }

    pub fn matches(&self, page_url: &str) -> String {
        self.0.matches(page_url).to_string()
    }

    pub fn credentials_for(&self, id: &str, page_url: &str) -> Res<String> {
        Ok(self.0.credentials_for(id, page_url).map_err(err)?.to_string())
    }

    pub fn totp(&self, id: &str, unix_secs: f64) -> Res<String> {
        Ok(self.0.totp(id, unix_secs as u64).map_err(err)?.to_string())
    }

    pub fn compact(&mut self, now_ms: f64, max_age_ms: f64) {
        self.0.compact(now_ms as u64, max_age_ms as u64);
    }

    pub fn export_csv(&self) -> Res<String> {
        Ok(self.0.export_csv().map_err(err)?.to_string())
    }

    pub fn export_encrypted(&self, password: &str, m_kib: u32, t: u32, p: u32) -> Res<Vec<u8>> {
        self.0.export_encrypted(password, kdf(m_kib, t, p), &mut rng()).map_err(err)
    }
}
