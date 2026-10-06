//! IPC commands: thin wrappers over `scytale_core`. Errors cross as readable strings.

use std::sync::atomic::Ordering;
use std::time::Duration;

use getrandom::SysRng;
use rand_core::UnwrapErr;
use scytale_core::session::{self, GeneratorRequest, Session};
use scytale_core::{DeviceId, Header, Hlc, Item, KdfParams, SecretKey, export, import};
use serde::Deserialize;
use serde_json::{Value, json};
use tauri::ipc::Response;
use tauri::{AppHandle, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::DialogExt;

use crate::AppState;

type Res<T> = Result<T, String>;

fn rng() -> UnwrapErr<SysRng> {
    UnwrapErr(SysRng)
}

fn s(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[derive(Deserialize)]
pub struct Kdf {
    m_kib: u32,
    t: u32,
    p: u32,
}

impl From<Kdf> for KdfParams {
    fn from(k: Kdf) -> Self {
        KdfParams { m_kib: k.m_kib, t: k.t, p: k.p }
    }
}

fn with_session<T>(state: &State<AppState>, f: impl FnOnce(&mut Session) -> Res<T>) -> Res<T> {
    let mut guard = state.session.lock().map_err(s)?;
    let session = guard.as_mut().ok_or("The vault is locked.")?;
    f(session)
}

#[tauri::command]
pub fn platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
}

#[tauri::command]
pub fn default_kdf() -> Value {
    let k = KdfParams::DEFAULT;
    json!({ "m_kib": k.m_kib, "t": k.t, "p": k.p })
}

#[tauri::command]
pub fn random_id() -> String {
    DeviceId::random(&mut rng()).to_hex()
}

#[tauri::command]
pub fn generate_secret_key() -> String {
    SecretKey::generate(&mut rng()).to_display().to_string()
}

#[tauri::command]
pub fn normalize_secret_key(input: String) -> Res<String> {
    Ok(SecretKey::parse(&input).map_err(s)?.to_display().to_string())
}

#[tauri::command]
pub fn header_vault_id(header: Vec<u8>) -> Res<String> {
    Ok(Header::decode(&header).map_err(s)?.vault_id.to_hex())
}

#[tauri::command]
pub fn header_supersedes(a: Vec<u8>, b: Vec<u8>) -> Res<bool> {
    Ok(Header::decode(&a).map_err(s)?.supersedes(&Header::decode(&b).map_err(s)?))
}

#[tauri::command]
pub fn generate(options: GeneratorRequest) -> Res<Value> {
    session::generate(options, &mut rng()).map_err(s)
}

#[tauri::command]
pub fn import_csv(text: String) -> Res<Value> {
    let r = import::csv(&text).map_err(s)?;
    Ok(json!({ "items": r.items, "skipped": r.skipped }))
}

#[tauri::command]
pub fn import_bitwarden_json(text: String) -> Res<Value> {
    let r = import::bitwarden_json(&text).map_err(s)?;
    Ok(json!({ "items": r.items, "skipped": r.skipped }))
}

#[tauri::command]
pub fn open_encrypted_export(bytes: Vec<u8>, password: String) -> Res<Vec<Item>> {
    export::open_encrypted(&bytes, &password).map_err(s)
}

#[tauri::command]
pub fn create_vault(
    state: State<AppState>,
    password: String,
    secret_key: String,
    kdf: Kdf,
    now_ms: u64,
    device: String,
) -> Res<Value> {
    let sk = SecretKey::parse(&secret_key).map_err(s)?;
    let device = DeviceId::from_hex(&device).map_err(s)?;
    let written = Hlc { wall_ms: now_ms, counter: 0, device };
    let (header, key) = Header::create(&password, &sk, kdf.into(), written, &mut rng()).map_err(s)?;
    *state.session.lock().map_err(s)? = Some(Session::new(key, header.vault_id, device));
    Ok(json!({ "header": header.encode(), "vaultId": header.vault_id.to_hex() }))
}

#[tauri::command]
pub fn unlock(
    state: State<AppState>,
    header: Vec<u8>,
    password: String,
    secret_key: String,
    device: String,
) -> Res<()> {
    let header = Header::decode(&header).map_err(s)?;
    let sk = SecretKey::parse(&secret_key).map_err(s)?;
    let key = header.unlock(&password, &sk).map_err(s)?;
    let device = DeviceId::from_hex(&device).map_err(s)?;
    *state.session.lock().map_err(s)? = Some(Session::new(key, header.vault_id, device));
    Ok(())
}

/// Drops the session: the key and every decrypted item are zeroized.
#[tauri::command]
pub fn lock(state: State<AppState>) -> Res<()> {
    *state.session.lock().map_err(s)? = None;
    Ok(())
}

#[tauri::command]
pub fn is_open(state: State<AppState>) -> Res<bool> {
    Ok(state.session.lock().map_err(s)?.is_some())
}

#[tauri::command]
pub fn change_password(
    state: State<AppState>,
    header: Vec<u8>,
    new_password: String,
    secret_key: String,
    kdf: Kdf,
    now_ms: u64,
    device: String,
) -> Res<Vec<u8>> {
    let header = Header::decode(&header).map_err(s)?;
    let sk = SecretKey::parse(&secret_key).map_err(s)?;
    let device = DeviceId::from_hex(&device).map_err(s)?;
    let written = Hlc { wall_ms: now_ms, counter: 0, device };
    with_session(&state, |session| {
        Ok(header
            .change_password(session.key(), &new_password, &sk, kdf.into(), written, &mut rng())
            .map_err(s)?
            .encode())
    })
}

#[tauri::command]
pub fn set_guard(state: State<AppState>, json: String) -> Res<()> {
    with_session(&state, |v| v.set_guard(&json).map_err(s))
}

#[tauri::command]
pub fn guard_json(state: State<AppState>) -> Res<String> {
    with_session(&state, |v| Ok(v.guard_json()))
}

#[tauri::command]
pub fn merge_snapshot(state: State<AppState>, bytes: Vec<u8>) -> Res<Value> {
    with_session(&state, |v| v.merge_snapshot(&bytes).map_err(s))
}

#[tauri::command]
pub fn seal(state: State<AppState>, seq: u64, device_name: String) -> Res<Response> {
    with_session(&state, |v| Ok(Response::new(v.seal(seq, &device_name, &mut rng()).map_err(s)?)))
}

#[tauri::command]
pub fn list(state: State<AppState>) -> Res<Value> {
    with_session(&state, |v| Ok(v.list()))
}

#[tauri::command]
pub fn view(state: State<AppState>, id: String) -> Res<Value> {
    with_session(&state, |v| v.view(&id).map_err(s))
}

#[tauri::command]
pub fn reveal(state: State<AppState>, id: String, field: String) -> Res<String> {
    with_session(&state, |v| v.reveal(&id, &field).map_err(s))
}

#[tauri::command]
pub fn reveal_history(state: State<AppState>, id: String, index: usize) -> Res<String> {
    with_session(&state, |v| v.reveal_history(&id, index).map_err(s))
}

#[tauri::command]
pub fn draft(state: State<AppState>, id: String) -> Res<Value> {
    with_session(&state, |v| v.draft(&id).map_err(s))
}

#[tauri::command]
pub fn upsert(state: State<AppState>, id: Option<String>, item: Item, now_ms: u64) -> Res<String> {
    with_session(&state, |v| v.upsert(id.as_deref(), item, now_ms, &mut rng()).map_err(s))
}

#[tauri::command]
pub fn add_items(state: State<AppState>, items: Vec<Item>, now_ms: u64) -> Res<usize> {
    with_session(&state, |v| Ok(v.add_items(items, now_ms, &mut rng())))
}

#[tauri::command]
pub fn remove(state: State<AppState>, id: String, now_ms: u64) -> Res<bool> {
    with_session(&state, |v| v.remove(&id, now_ms).map_err(s))
}

#[tauri::command]
pub fn matches(state: State<AppState>, page_url: String) -> Res<Value> {
    with_session(&state, |v| Ok(v.matches(&page_url)))
}

#[tauri::command]
pub fn credentials_for(state: State<AppState>, id: String, page_url: String) -> Res<Value> {
    with_session(&state, |v| v.credentials_for(&id, &page_url).map_err(s))
}

#[tauri::command]
pub fn totp(state: State<AppState>, id: String, unix_secs: u64) -> Res<Value> {
    with_session(&state, |v| v.totp(&id, unix_secs).map_err(s))
}

#[tauri::command]
pub fn compact(state: State<AppState>, now_ms: u64, max_age_ms: u64) -> Res<()> {
    with_session(&state, |v| {
        v.compact(now_ms, max_age_ms);
        Ok(())
    })
}

#[tauri::command]
pub fn export_csv(state: State<AppState>) -> Res<Value> {
    with_session(&state, |v| v.export_csv().map_err(s))
}

#[tauri::command]
pub fn export_encrypted(state: State<AppState>, password: String, kdf: Kdf) -> Res<Response> {
    with_session(&state, |v| Ok(Response::new(v.export_encrypted(&password, kdf.into(), &mut rng()).map_err(s)?)))
}

// --- OS credential store: the Secret Key at rest --------------------------------------------

const KEYRING_SERVICE: &str = "Scytale";

#[tauri::command]
pub fn keyring_get(account: String) -> Res<Option<String>> {
    match keyring::Entry::new(KEYRING_SERVICE, &account).map_err(s)?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(s(e)),
    }
}

#[tauri::command]
pub fn keyring_set(account: String, value: String) -> Res<()> {
    keyring::Entry::new(KEYRING_SERVICE, &account).map_err(s)?.set_password(&value).map_err(s)
}

#[tauri::command]
pub fn keyring_delete(account: String) -> Res<()> {
    match keyring::Entry::new(KEYRING_SERVICE, &account).map_err(s)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(s(e)),
    }
}

// --- Clipboard ----------------------------------------------------------------------------------

/// Copies `text`; with `clear_after_secs > 0`, clears it later, but only if it is still ours.
#[tauri::command]
pub fn copy_text(app: AppHandle, state: State<AppState>, text: String, clear_after_secs: u64) -> Res<()> {
    app.clipboard().write_text(text.clone()).map_err(s)?;
    let generation = state.clip_generation.fetch_add(1, Ordering::SeqCst) + 1;
    if clear_after_secs > 0 {
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(clear_after_secs));
            let state = tauri::Manager::state::<AppState>(&app);
            let ours = app.clipboard().read_text().is_ok_and(|current| current == text);
            if state.clip_generation.load(Ordering::SeqCst) == generation && ours {
                let _ = app.clipboard().write_text(String::new());
            }
        });
    }
    Ok(())
}

// --- Files: dialogs run here, so the webview never gets filesystem access -------------------

#[tauri::command]
pub async fn pick_import_file(app: AppHandle) -> Res<Option<Value>> {
    let Some(path) = app.dialog().file().add_filter("Password exports", &["csv", "json", "scyx"]).blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = path.into_path().map_err(s)?;
    let bytes = std::fs::read(&path).map_err(s)?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(Some(json!({ "name": name, "bytes": bytes })))
}

/// Asks where to save, then writes. Returns false if the user cancelled.
#[tauri::command]
pub async fn save_file(app: AppHandle, suggested_name: String, bytes: Vec<u8>) -> Res<bool> {
    let Some(path) = app.dialog().file().set_file_name(&suggested_name).blocking_save_file() else {
        return Ok(false);
    };
    std::fs::write(path.into_path().map_err(s)?, bytes).map_err(s)?;
    Ok(true)
}

#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Res<Option<String>> {
    Ok(app
        .dialog()
        .file()
        .blocking_pick_folder()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned()))
}
