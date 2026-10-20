mod bench;
mod config;
mod documents;
mod highlight;
mod menu;
pub mod render;
mod settings;
mod watch;

use std::path::PathBuf;

use tauri::{AppHandle, Manager, RunEvent, WindowEvent};

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
        .manage(documents::Renders::default())
        .manage(config::Config::load())
        .manage(watch::Watchers::default())
        .manage(menu::Zoom::default())
        .menu(menu::build)
        .on_menu_event(|app, event| menu::handle(app, &event))
        .invoke_handler(tauri::generate_handler![
            documents::get_document,
            documents::follow_link,
            settings::get_settings,
            settings::set_setting,
            settings::reveal_config,
            bench::bench_painted
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
            app.manage(watch_config(app));
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
                app.state::<menu::Zoom>().forget(&label);
                app.state::<documents::Paths>().remove(&label);
                app.state::<documents::Renders>().remove(&label);
            }
            WindowEvent::Focused(true) => {
                if let Some(window) = app.get_webview_window(&label) {
                    menu::update(app, &window);
                }
            }
            WindowEvent::ThemeChanged(theme) => {
                if let Some(window) = app
                    .get_webview_window(&label)
                    .filter(|_| label != settings::LABEL)
                {
                    documents::paint_background(&window, theme);
                }
            }
            _ => {}
        },
        _ => {}
    });
}

/// Kept alive by the app state: dropping it would stop the watch.
struct ConfigWatcher {
    _watcher: Option<watch::Watcher>,
}

/// Restyles every window when the config file changes.
fn watch_config(app: &AppHandle) -> ConfigWatcher {
    let Some(path) = config::path() else {
        return ConfigWatcher { _watcher: None };
    };
    // The directory must exist for its future config file to be noticed.
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }

    let app = app.clone();
    let watcher = watch::on_change(&path, move || {
        if let Some(style) = app.state::<config::Config>().reload() {
            documents::restyle(&app, &style);
        }
    });
    ConfigWatcher { _watcher: watcher }
}

/// Files passed to the binary itself (`target/release/mdr file.md`).
fn cli_paths() -> Vec<PathBuf> {
    std::env::args_os()
        .skip(1)
        .filter(|arg| !arg.to_string_lossy().starts_with('-'))
        .map(PathBuf::from)
        .collect()
}
