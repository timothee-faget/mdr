//! Document windows: one window per file, keyed by its canonical path.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        Mutex, PoisonError,
        atomic::{AtomicUsize, Ordering},
    },
};

use serde::Serialize;
use tauri::{
    AppHandle, LogicalPosition, Manager, State, Theme, TitleBarStyle, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, window::Color,
};
use tauri_plugin_dialog::DialogExt;

use crate::render;

const EXTENSIONS: &[&str] = &["md", "markdown", "mdown", "mkd"];

/// Offset of a new window from the focused one, like macOS document apps.
const CASCADE: f64 = 24.0;

/// Which file each document window shows, by window label.
#[derive(Default)]
pub struct Paths(Mutex<HashMap<String, PathBuf>>);

impl Paths {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, PathBuf>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn get(&self, label: &str) -> Option<PathBuf> {
        self.lock().get(label).cloned()
    }

    fn label_of(&self, path: &Path) -> Option<String> {
        self.lock()
            .iter()
            .find(|(_, shown)| *shown == path)
            .map(|(label, _)| label.clone())
    }

    fn insert(&self, label: String, path: PathBuf) {
        self.lock().insert(label, path);
    }

    pub fn remove(&self, label: &str) {
        self.lock().remove(label);
    }
}

#[derive(Serialize)]
pub struct Document {
    title: String,
    path: PathBuf,
    html: String,
}

/// Opens `path` in a new window, or focuses the window already showing it.
pub fn open(app: &AppHandle, path: &Path) {
    let path = resolve(path);
    let paths = app.state::<Paths>();

    if let Some(window) = paths
        .label_of(&path)
        .and_then(|label| app.get_webview_window(&label))
    {
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }

    let label = next_label();
    paths.insert(label.clone(), path.clone());
    build_window(app, &label, &file_name(&path));
}

/// Opens a window with no document.
pub fn open_empty(app: &AppHandle) {
    build_window(app, &next_label(), "mdr");
}

/// Asks for a file. It replaces the focused window if that one is empty,
/// otherwise it opens in a new window.
pub fn pick(app: &AppHandle) {
    let focused = focused_window(app);

    let mut dialog = app.dialog().file().add_filter("Markdown", EXTENSIONS);
    if let Some(window) = &focused {
        dialog = dialog.set_parent(window);
    }

    let app = app.clone();
    dialog.pick_file(move |file| {
        let Some(path) = file.and_then(|file| file.into_path().ok()) else {
            return;
        };
        match focused {
            Some(window) if is_empty(&app, &window) => show_in(&app, &window, &path),
            _ => open(&app, &path),
        }
    });
}

/// Paints the window background in the theme's color, so no white flashes
/// before the page loads or when scrolling past its edges.
pub fn paint_background(window: &WebviewWindow, theme: Theme) {
    // Keep in sync with `--bg` in ui/index.html, until the config (#6) drives both.
    let color = match theme {
        Theme::Dark => Color(0x1c, 0x1c, 0x1e, 0xff),
        _ => Color(0xfb, 0xfa, 0xf7, 0xff),
    };
    let _ = window.set_background_color(Some(color));
}

/// The document shown by the calling window, or `None` for an empty window.
#[tauri::command]
pub async fn get_document(
    window: WebviewWindow,
    paths: State<'_, Paths>,
) -> Result<Option<Document>, String> {
    let Some(path) = paths.get(window.label()) else {
        return Ok(None);
    };
    let title = file_name(&path);
    let bytes = fs::read(&path).map_err(|error| format!("Can't open {title}: {error}"))?;

    Ok(Some(Document {
        html: render::to_html(&String::from_utf8_lossy(&bytes)),
        title,
        path,
    }))
}

fn is_empty(app: &AppHandle, window: &WebviewWindow) -> bool {
    app.state::<Paths>().get(window.label()).is_none()
}

/// Loads `path` into an empty window, unless another window already shows it.
fn show_in(app: &AppHandle, window: &WebviewWindow, path: &Path) {
    let path = resolve(path);
    let paths = app.state::<Paths>();
    if paths.label_of(&path).is_some() {
        open(app, &path);
        return;
    }

    let _ = window.set_title(&file_name(&path));
    paths.insert(window.label().to_owned(), path);
    let _ = window.reload();
}

fn build_window(app: &AppHandle, label: &str, title: &str) {
    let mut builder = WebviewWindowBuilder::new(app, label, WebviewUrl::default())
        .title(title)
        .inner_size(760.0, 900.0)
        .title_bar_style(TitleBarStyle::Overlay)
        .hidden_title(true);
    if let Some(position) = cascade_position(app) {
        builder = builder.position(position.x, position.y);
    }

    match builder.build() {
        Ok(window) => paint_background(&window, window.theme().unwrap_or(Theme::Light)),
        Err(error) => eprintln!("mdr: can't create a window: {error}"),
    }
}

fn cascade_position(app: &AppHandle) -> Option<LogicalPosition<f64>> {
    let focused = focused_window(app)?;
    let scale = focused.scale_factor().ok()?;
    let position = focused.outer_position().ok()?.to_logical::<f64>(scale);
    Some(LogicalPosition::new(
        position.x + CASCADE,
        position.y + CASCADE,
    ))
}

fn focused_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.webview_windows()
        .into_values()
        .find(|window| window.is_focused().unwrap_or(false))
}

fn next_label() -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!("doc-{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

/// Canonical path when the file exists, so that symlinks and `..` open the
/// same window. A missing file keeps its absolute path to report the error.
fn resolve(path: &Path) -> PathBuf {
    fs::canonicalize(path)
        .or_else(|_| std::path::absolute(path))
        .unwrap_or_else(|_| path.to_owned())
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "tests/fixtures/code.md";

    fn paths_showing(path: &Path) -> Paths {
        let paths = Paths::default();
        paths.insert("doc-0".to_owned(), resolve(path));
        paths
    }

    #[test]
    fn relative_and_absolute_paths_share_a_window() {
        let paths = paths_showing(&fs::canonicalize(FIXTURE).unwrap());
        let roundabout = Path::new("tests/fixtures/../fixtures/code.md");

        assert_eq!(
            paths.label_of(&resolve(roundabout)).as_deref(),
            Some("doc-0")
        );
    }

    #[test]
    fn symlinks_share_a_window_with_their_target() {
        let paths = paths_showing(Path::new(FIXTURE));
        let link = std::env::temp_dir().join(format!("mdr-test-{}.md", std::process::id()));
        let _ = fs::remove_file(&link);
        std::os::unix::fs::symlink(fs::canonicalize(FIXTURE).unwrap(), &link).unwrap();

        let label = paths.label_of(&resolve(&link));
        fs::remove_file(&link).unwrap();
        assert_eq!(label.as_deref(), Some("doc-0"));
    }

    #[test]
    fn different_files_get_different_windows() {
        let paths = paths_showing(Path::new(FIXTURE));

        assert_eq!(
            paths.label_of(&resolve(Path::new("tests/fixtures/lists.md"))),
            None
        );
    }

    #[test]
    fn closed_windows_are_forgotten() {
        let paths = paths_showing(Path::new(FIXTURE));
        paths.remove("doc-0");

        assert_eq!(paths.label_of(&resolve(Path::new(FIXTURE))), None);
    }

    #[test]
    fn missing_files_resolve_to_an_absolute_path() {
        let path = resolve(Path::new("does-not-exist.md"));

        assert!(path.is_absolute());
        assert_eq!(file_name(&path), "does-not-exist.md");
    }

    #[test]
    fn window_labels_are_unique() {
        assert_ne!(next_label(), next_label());
    }
}
