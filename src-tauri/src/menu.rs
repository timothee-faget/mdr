//! The menu bar: every action and shortcut of the app.

use std::{
    collections::HashMap,
    fs,
    sync::{Mutex, PoisonError},
};

use tauri::{
    AppHandle, Emitter, Manager, WebviewWindow, Wry,
    menu::{AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
};
use tauri_plugin_opener::OpenerExt;

use crate::{config, documents};

const OPEN: &str = "open";
const REVEAL: &str = "reveal";
const ACTUAL_SIZE: &str = "actual-size";
const ZOOM_IN: &str = "zoom-in";
const ZOOM_OUT: &str = "zoom-out";
const RELOAD: &str = "reload";
const OPEN_CONFIG: &str = "open-config";
const FIND: &str = "find";
const FIND_NEXT: &str = "find-next";
const FIND_PREVIOUS: &str = "find-previous";

/// Zoom steps, as in Safari.
const ZOOM_LEVELS: &[f64] = &[
    0.5, 0.67, 0.75, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0,
];

/// Written by File › Open Config File when there is no config yet.
const CONFIG_TEMPLATE: &str = r##"# mdr configuration. Changes apply as soon as the file is saved.
# Every key is optional: uncomment and edit.

# appearance = "auto"        # auto | light | dark
# light_theme = "paper"      # paper, sepia, or a [themes.<name>] below
# dark_theme = "night"       # night, or a [themes.<name>] below

# [font]
# family = "-apple-system, system-ui, sans-serif"   # a CSS font-family list
# mono = "ui-monospace, SF Mono, Menlo, monospace"
# size = 16
# line_height = 1.6

# [layout]
# max_width = "70ch"         # ch, px, em, rem or %

# [themes.mine]              # unset colors come from paper
# background = "#f7f3ea"     # #rrggbb or rgb()
# text = "#2b2b2b"
# muted = "#7a7468"
# accent = "#9c4a1a"
# code_background = "#efe9dc"
# border = "#e2dccd"
# code_theme = "InspiredGitHub"   # code colors, also for built-in themes:
#   InspiredGitHub, GitHub, Solarized (light), Solarized (dark), OneHalfLight,
#   OneHalfDark, Nord, Dracula, gruvbox-light, gruvbox-dark, Catppuccin Latte,
#   Catppuccin Mocha, Monokai Extended, TwoDark, zenburn…
"##;

/// Menu items that need a document, disabled in empty windows.
pub struct DocumentItems([MenuItem<Wry>; 5]);

/// Zoom level of each window, by label. Missing means 1.
#[derive(Default)]
pub struct Zoom(Mutex<HashMap<String, f64>>);

impl Zoom {
    pub fn forget(&self, label: &str) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(label);
    }
}

pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let separator = || PredefinedMenuItem::separator(app);
    let item = |id: &str, text: &str, shortcut: Option<&str>| {
        MenuItem::with_id(app, id, text, true, shortcut)
    };

    let reveal = MenuItem::with_id(app, REVEAL, "Reveal in Finder", false, None::<&str>)?;
    let reload = MenuItem::with_id(app, RELOAD, "Reload", false, Some("CmdOrCtrl+R"))?;
    let find = MenuItem::with_id(app, FIND, "Find…", false, Some("CmdOrCtrl+F"))?;
    let find_next = MenuItem::with_id(app, FIND_NEXT, "Find Next", false, Some("CmdOrCtrl+G"))?;
    let find_previous = MenuItem::with_id(
        app,
        FIND_PREVIOUS,
        "Find Previous",
        false,
        Some("CmdOrCtrl+Shift+G"),
    )?;

    let package = app.package_info();
    let about = AboutMetadata {
        name: Some(package.name.clone()),
        version: Some(package.version.to_string()),
        ..AboutMetadata::default()
    };

    let app_menu = Submenu::with_items(
        app,
        &package.name,
        true,
        &[
            &PredefinedMenuItem::about(app, None, Some(about))?,
            &separator()?,
            &PredefinedMenuItem::services(app, None)?,
            &separator()?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &separator()?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;
    let file = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &item(OPEN, "Open…", Some("CmdOrCtrl+O"))?,
            &separator()?,
            &reveal,
            &item(OPEN_CONFIG, "Open Config File", None)?,
            &separator()?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;
    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
            &separator()?,
            &find,
            &find_next,
            &find_previous,
        ],
    )?;
    let view = Submenu::with_items(
        app,
        "View",
        true,
        &[
            &item(ACTUAL_SIZE, "Actual Size", Some("CmdOrCtrl+0"))?,
            &item(ZOOM_IN, "Zoom In", Some("CmdOrCtrl+="))?,
            &item(ZOOM_OUT, "Zoom Out", Some("CmdOrCtrl+-"))?,
            &separator()?,
            &reload,
            &separator()?,
            &PredefinedMenuItem::fullscreen(app, None)?,
        ],
    )?;
    let window = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            &separator()?,
            &PredefinedMenuItem::bring_all_to_front(app, None)?,
        ],
    )?;
    // macOS lists the open windows here, and adds its search field to Help.
    window.set_as_windows_menu_for_nsapp()?;
    let help = Submenu::with_items(app, "Help", true, &[])?;
    help.set_as_help_menu_for_nsapp()?;

    app.manage(DocumentItems([
        reveal,
        reload,
        find,
        find_next,
        find_previous,
    ]));
    Menu::with_items(app, &[&app_menu, &file, &edit, &view, &window, &help])
}

pub fn handle(app: &AppHandle, event: &MenuEvent) {
    match event.id().as_ref() {
        OPEN => documents::pick(app),
        OPEN_CONFIG => open_config(app),
        id => {
            let Some(window) = documents::focused_window(app) else {
                return;
            };
            match id {
                REVEAL => documents::reveal(&window),
                RELOAD => {
                    let _ = window.emit_to(window.label(), "document-changed", ());
                }
                FIND => find(&window, "open"),
                FIND_NEXT => find(&window, "next"),
                FIND_PREVIOUS => find(&window, "previous"),
                ACTUAL_SIZE => zoom(app, &window, 0),
                ZOOM_IN => zoom(app, &window, 1),
                ZOOM_OUT => zoom(app, &window, -1),
                _ => {}
            }
        }
    }
}

/// Enables the document items when `window` shows a document.
pub fn update(app: &AppHandle, window: &WebviewWindow) {
    let enabled = documents::path_of(app, window.label()).is_some();
    for item in &app.state::<DocumentItems>().0 {
        let _ = item.set_enabled(enabled);
    }
}

/// Opens the search bar or moves between matches (ui/search.js).
fn find(window: &WebviewWindow, action: &str) {
    let _ = window.emit_to(window.label(), "find", action);
}

/// Zooms the window one step in (`1`) or out (`-1`), or resets it (`0`).
fn zoom(app: &AppHandle, window: &WebviewWindow, direction: i8) {
    let zoom = app.state::<Zoom>();
    let mut levels = zoom.0.lock().unwrap_or_else(PoisonError::into_inner);
    let current = levels.get(window.label()).copied().unwrap_or(1.0);
    let next = next_zoom(current, direction);
    if window.set_zoom(next).is_ok() {
        levels.insert(window.label().to_owned(), next);
    }
}

fn next_zoom(current: f64, direction: i8) -> f64 {
    match direction {
        0 => 1.0,
        1 => ZOOM_LEVELS
            .iter()
            .copied()
            .find(|&level| level > current + f64::EPSILON)
            .unwrap_or(current),
        _ => ZOOM_LEVELS
            .iter()
            .rev()
            .copied()
            .find(|&level| level < current - f64::EPSILON)
            .unwrap_or(current),
    }
}

/// Opens the config file in the default app, creating a commented template
/// first if there is none.
fn open_config(app: &AppHandle) {
    let Some(path) = config::path() else {
        return;
    };
    if !path.exists() {
        let written = path
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| fs::write(&path, CONFIG_TEMPLATE));
        if let Err(error) = written {
            eprintln!("mdr: can't create {}: {error}", path.display());
            return;
        }
    }
    if let Err(error) = app.opener().open_path(path.to_string_lossy(), None::<&str>) {
        eprintln!("mdr: can't open {}: {error}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_steps_through_the_levels() {
        assert!((next_zoom(1.0, 1) - 1.1).abs() < f64::EPSILON);
        assert!((next_zoom(1.0, -1) - 0.9).abs() < f64::EPSILON);
        assert!((next_zoom(2.5, 0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn zoom_stops_at_the_bounds() {
        assert!((next_zoom(3.0, 1) - 3.0).abs() < f64::EPSILON);
        assert!((next_zoom(0.5, -1) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn config_template_parses_to_the_defaults() {
        assert_eq!(
            config::tests::css_of(CONFIG_TEMPLATE),
            config::tests::css_of("")
        );
    }
}
