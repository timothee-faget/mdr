mod config;
mod documents;
pub mod render;
mod watch;

use std::path::PathBuf;

use tauri::{
    AppHandle, Manager, RunEvent, WindowEvent,
    menu::{Menu, MenuItem, PredefinedMenuItem},
};

const OPEN_MENU_ID: &str = "open";

/// Starts the app.
///
/// # Panics
///
/// If the Tauri app can't be built, which only happens with an invalid
/// `tauri.conf.json`.
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(documents::Paths::default())
        .manage(config::load())
        .manage(watch::Watchers::default())
        .menu(menu)
        .on_menu_event(|app, event| {
            if event.id() == OPEN_MENU_ID {
                documents::pick(app);
            }
        })
        .invoke_handler(tauri::generate_handler![
            documents::get_document,
            documents::follow_link
        ])
        .build(tauri::generate_context!())
        .expect("error while building mdr");

    // On launch, macOS delivers the files to open before the app is ready:
    // queue them until `Ready`, then decide whether an empty window is needed.
    let mut pending = Some(cli_paths());

    app.run(move |app, event| match event {
        RunEvent::Opened { urls } => {
            let paths = urls.iter().filter_map(|url| url.to_file_path().ok());
            match &mut pending {
                Some(queue) => queue.extend(paths),
                None => paths.for_each(|path| documents::open(app, &path)),
            }
        }
        RunEvent::Ready => {
            let paths = pending.take().unwrap_or_default();
            if paths.is_empty() {
                documents::open_empty(app);
            }
            for path in paths {
                documents::open(app, &path);
            }
        }
        // Like any Mac app, keep running when the last window closes.
        // An explicit quit carries an exit code and goes through.
        RunEvent::ExitRequested {
            code: None, api, ..
        } => api.prevent_exit(),
        // Dock icon clicked while every window is closed.
        RunEvent::Reopen {
            has_visible_windows: false,
            ..
        } => documents::open_empty(app),
        RunEvent::WindowEvent { label, event, .. } => match event {
            WindowEvent::Destroyed => {
                app.state::<watch::Watchers>().stop(&label);
                app.state::<documents::Paths>().remove(&label);
            }
            WindowEvent::ThemeChanged(theme) => {
                if let Some(window) = app.get_webview_window(&label) {
                    documents::paint_background(&window, theme);
                }
            }
            _ => {}
        },
        _ => {}
    });
}

/// Tauri's default macOS menu, plus File › Open… (⌘O).
fn menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::default(app)?;
    // The default menu is: app name, File, Edit, View, Window, Help.
    if let Some(file) = menu.items()?.get(1).and_then(|item| item.as_submenu()) {
        file.insert_items(
            &[
                &MenuItem::with_id(app, OPEN_MENU_ID, "Open…", true, Some("CmdOrCtrl+O"))?,
                &PredefinedMenuItem::separator(app)?,
            ],
            0,
        )?;
    }
    Ok(menu)
}

/// Files passed to the binary itself (`target/release/mdr file.md`).
fn cli_paths() -> Vec<PathBuf> {
    std::env::args_os()
        .skip(1)
        .filter(|arg| !arg.to_string_lossy().starts_with('-'))
        .map(PathBuf::from)
        .collect()
}
