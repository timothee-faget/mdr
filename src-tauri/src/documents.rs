//! Document windows: one window per file, keyed by its canonical path.

use std::{
    collections::HashMap,
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    sync::{Mutex, PoisonError},
};

use serde::Serialize;
use tauri::{
    AppHandle, Manager, State, TitleBarStyle, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

use crate::render;

/// Which file each document window shows, by window label.
#[derive(Default)]
pub struct Paths(Mutex<HashMap<String, PathBuf>>);

impl Paths {
    fn get(&self, label: &str) -> Option<PathBuf> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(label)
            .cloned()
    }

    fn insert(&self, label: String, path: PathBuf) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(label, path);
    }

    pub fn remove(&self, label: &str) {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(label);
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
    let label = label_for(&path);

    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }

    app.state::<Paths>().insert(label.clone(), path.clone());
    if let Err(error) = window(app, &label).title(file_name(&path)).build() {
        eprintln!("mdr: can't create a window for {}: {error}", path.display());
    }
}

/// Opens a window with no document.
pub fn open_empty(app: &AppHandle) {
    if let Err(error) = window(app, "empty").title("mdr").build() {
        eprintln!("mdr: can't create a window: {error}");
    }
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

fn window<'a>(app: &'a AppHandle, label: &str) -> WebviewWindowBuilder<'a, tauri::Wry, AppHandle> {
    WebviewWindowBuilder::new(app, label, WebviewUrl::default())
        .inner_size(760.0, 900.0)
        .title_bar_style(TitleBarStyle::Overlay)
        .hidden_title(true)
}

/// Canonical path when the file exists, so that symlinks and `..` open the
/// same window. A missing file keeps its absolute path to report the error.
fn resolve(path: &Path) -> PathBuf {
    fs::canonicalize(path)
        .or_else(|_| std::path::absolute(path))
        .unwrap_or_else(|_| path.to_owned())
}

fn label_for(path: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    format!("doc-{:016x}", hasher.finish())
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

    #[test]
    fn relative_and_absolute_paths_share_a_window() {
        let absolute = fs::canonicalize(FIXTURE).unwrap();
        let roundabout = Path::new("tests/fixtures/../fixtures/code.md");

        assert_eq!(
            label_for(&resolve(roundabout)),
            label_for(&resolve(&absolute))
        );
    }

    #[test]
    fn symlinks_share_a_window_with_their_target() {
        let link = std::env::temp_dir().join(format!("mdr-test-{}.md", std::process::id()));
        let _ = fs::remove_file(&link);
        std::os::unix::fs::symlink(fs::canonicalize(FIXTURE).unwrap(), &link).unwrap();

        let same = label_for(&resolve(&link)) == label_for(&resolve(Path::new(FIXTURE)));
        fs::remove_file(&link).unwrap();
        assert!(same);
    }

    #[test]
    fn different_files_get_different_windows() {
        assert_ne!(
            label_for(&resolve(Path::new(FIXTURE))),
            label_for(&resolve(Path::new("tests/fixtures/lists.md")))
        );
    }

    #[test]
    fn missing_files_resolve_to_an_absolute_path() {
        let path = resolve(Path::new("does-not-exist.md"));

        assert!(path.is_absolute());
        assert_eq!(file_name(&path), "does-not-exist.md");
    }
}
