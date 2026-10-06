//! Scytale desktop. The vault key lives only here, in Rust memory, and is wiped on lock; the webview
//! gets item summaries and the individual secrets the user reveals or copies.

mod commands;
mod folder;

use std::sync::Mutex;
use std::sync::atomic::AtomicU64;

use scytale_core::session::Session;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WindowEvent};

#[derive(Default)]
pub struct AppState {
    pub session: Mutex<Option<Session>>,
    /// Bumped on every copy, so an older clear-timer never wipes a newer copy.
    pub clip_generation: AtomicU64,
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn lock(app: &AppHandle) {
    if let Ok(mut s) = app.state::<AppState>().session.lock() {
        *s = None;
    }
    let _ = app.emit("vault-locked", ());
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // A second launch focuses the running app instead of opening another copy.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState::default())
        .setup(|app| {
            let open = MenuItem::with_id(app, "open", "Open Scytale", true, None::<&str>)?;
            let lock_item = MenuItem::with_id(app, "lock", "Lock", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Scytale", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &lock_item, &quit])?;
            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().expect("bundled icon").clone())
                .tooltip("Scytale")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_main(app),
                    "lock" => lock(app),
                    "quit" => {
                        lock(app);
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left, button_state: MouseButtonState::Up, ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;

            // Started at login: wait in the tray, locked.
            if !std::env::args().any(|a| a == "--hidden") {
                show_main(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // The close button hides to the tray; quitting lives in the tray menu.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                let _ = window.emit("window-hidden", ());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::platform,
            commands::default_kdf,
            commands::random_id,
            commands::generate_secret_key,
            commands::normalize_secret_key,
            commands::header_vault_id,
            commands::header_supersedes,
            commands::generate,
            commands::import_csv,
            commands::import_bitwarden_json,
            commands::open_encrypted_export,
            commands::create_vault,
            commands::unlock,
            commands::lock,
            commands::is_open,
            commands::change_password,
            commands::set_guard,
            commands::guard_json,
            commands::merge_snapshot,
            commands::seal,
            commands::list,
            commands::view,
            commands::reveal,
            commands::reveal_history,
            commands::draft,
            commands::upsert,
            commands::add_items,
            commands::remove,
            commands::matches,
            commands::credentials_for,
            commands::totp,
            commands::compact,
            commands::export_csv,
            commands::export_encrypted,
            commands::keyring_get,
            commands::keyring_set,
            commands::keyring_delete,
            commands::copy_text,
            commands::pick_import_file,
            commands::save_file,
            commands::pick_folder,
            folder::folder_list,
            folder::folder_get,
            folder::folder_put,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Scytale");
}
