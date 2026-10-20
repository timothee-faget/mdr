//! The Settings window: a form over `config.toml`. Every change is written
//! to the file at once, and the config hot reload restyles the documents.

use std::fs;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

use crate::config::{self, Config, Settings};

pub const LABEL: &str = "settings";

/// Opens the Settings window, or focuses it if it's already open.
pub fn open(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.set_focus();
        return;
    }
    let built = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("settings.html".into()))
        .title("Settings")
        .inner_size(600.0, 720.0)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .theme(app.state::<Config>().current().appearance)
        .build();
    if let Err(error) = built {
        eprintln!("mdr: can't open the settings: {error}");
    }
}

/// The current config, resolved, for the form.
#[tauri::command]
pub fn get_settings() -> Settings {
    config::settings()
}

/// Writes one setting to the config file. `value: null` removes the key,
/// which brings back its default.
// Tauri commands receive their arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn set_setting(path: Vec<String>, value: serde_json::Value) -> Result<(), String> {
    let path: Vec<&str> = path.iter().map(String::as_str).collect();
    if !config::is_editable(&path) {
        return Err(format!("`{}` isn't a setting", path.join(".")));
    }
    let value = match value {
        serde_json::Value::Null => None,
        serde_json::Value::String(text) => Some(text.into()),
        serde_json::Value::Number(number) => Some(match number.as_i64() {
            Some(integer) => integer.into(),
            None => number.as_f64().unwrap_or_default().into(),
        }),
        other => return Err(format!("unsupported value {other}")),
    };

    let file = config::ensure_file().map_err(|error| error.to_string())?;
    let source = fs::read_to_string(&file).map_err(|error| error.to_string())?;
    let edited = config::edit(&source, &path, value)?;
    fs::write(&file, edited).map_err(|error| error.to_string())
}

/// Shows the config file in the Finder, creating it first if needed.
// Tauri commands receive their arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn reveal_config(app: AppHandle) -> Result<(), String> {
    let file = config::ensure_file().map_err(|error| error.to_string())?;
    app.opener()
        .reveal_item_in_dir(file)
        .map_err(|error| error.to_string())
}
