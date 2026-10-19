//! Reacts to files changing on disk: documents and the config file.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, PoisonError},
    time::Duration,
};

use notify_debouncer_full::{
    DebounceEventResult, Debouncer, RecommendedCache, new_debouncer,
    notify::{EventKind, RecommendedWatcher, RecursiveMode},
};
use tauri::{AppHandle, Emitter};

/// Saves are often several writes in a row: wait for them to settle.
const DEBOUNCE: Duration = Duration::from_millis(100);

/// Watches one file until dropped.
pub type Watcher = Debouncer<RecommendedWatcher, RecommendedCache>;

/// Calls `on_change` whenever `path` is written, replaced, created or
/// deleted.
///
/// The parent directory is watched rather than the file: many editors
/// save by writing a new file and renaming it over the old one, which
/// would end a watch on the file itself. Symlinks are resolved first, so
/// that a config file linked from a dotfiles repository is followed.
pub fn on_change(path: &Path, on_change: impl Fn() + Send + 'static) -> Option<Watcher> {
    let file = real_path(path);
    let dir = file.parent()?.to_owned();

    let watcher = new_debouncer(DEBOUNCE, None, move |result: DebounceEventResult| {
        if result.is_ok_and(|events| {
            events
                .iter()
                .any(|event| touches(event.kind, &event.paths, &file))
        }) {
            on_change();
        }
    })
    .and_then(|mut watcher| {
        watcher.watch(&dir, RecursiveMode::NonRecursive)?;
        Ok(watcher)
    });

    watcher
        .inspect_err(|error| eprintln!("mdr: can't watch {}: {error}", path.display()))
        .ok()
}

/// One watcher per document window, by window label.
#[derive(Default)]
pub struct Watchers(Mutex<HashMap<String, Watcher>>);

impl Watchers {
    /// Tells the window `label` to reload whenever `path` changes.
    pub fn watch(&self, app: &AppHandle, label: &str, path: &Path) {
        let (app, window) = (app.clone(), label.to_owned());
        let watcher = on_change(path, move || {
            let _ = app.emit_to(&window, "document-changed", ());
        });
        if let Some(watcher) = watcher {
            self.lock().insert(label.to_owned(), watcher);
        }
    }

    pub fn stop(&self, label: &str) {
        self.lock().remove(label);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Watcher>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// `path` with symlinks resolved, also when the file doesn't exist yet.
fn real_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| {
        match (path.parent().map(fs::canonicalize), path.file_name()) {
            (Some(Ok(dir)), Some(name)) => dir.join(name),
            _ => path.to_owned(),
        }
    })
}

/// Whether an event in the watched directory concerns the file. Reads don't
/// count: they include mdr's own.
fn touches(kind: EventKind, paths: &[PathBuf], file: &Path) -> bool {
    !matches!(kind, EventKind::Access(_)) && paths.iter().any(|path| path == file)
}

#[cfg(test)]
mod tests {
    use notify_debouncer_full::notify::event::{AccessKind, CreateKind, ModifyKind, RenameMode};

    use super::*;

    const FILE: &str = "/docs/notes.md";

    fn touches_file(kind: EventKind, path: &str) -> bool {
        touches(kind, &[PathBuf::from(path)], Path::new(FILE))
    }

    #[test]
    fn writes_and_atomic_saves_reload() {
        assert!(touches_file(EventKind::Modify(ModifyKind::Any), FILE));
        assert!(touches_file(EventKind::Create(CreateKind::File), FILE));
        assert!(touches_file(
            EventKind::Modify(ModifyKind::Name(RenameMode::To)),
            FILE
        ));
    }

    #[test]
    fn symlinks_are_followed_even_before_the_file_exists() {
        let root = std::env::temp_dir().join(format!("mdr-watch-{}", std::process::id()));
        let (dotfiles, config) = (root.join("dotfiles"), root.join("config"));
        fs::create_dir_all(&dotfiles).unwrap();
        fs::write(dotfiles.join("config.toml"), "").unwrap();
        std::os::unix::fs::symlink(&dotfiles, &config).unwrap();

        let target = fs::canonicalize(dotfiles.join("config.toml")).unwrap();
        let linked = real_path(&config.join("config.toml"));
        let missing = real_path(&config.join("missing.toml"));
        fs::remove_dir_all(&root).unwrap();

        assert_eq!(linked, target);
        assert_eq!(missing, target.with_file_name("missing.toml"));
    }

    #[test]
    fn other_files_and_reads_are_ignored() {
        assert!(!touches_file(
            EventKind::Modify(ModifyKind::Any),
            "/docs/other.md"
        ));
        assert!(!touches_file(
            EventKind::Modify(ModifyKind::Any),
            "/docs/.notes.md.swp"
        ));
        assert!(!touches_file(EventKind::Access(AccessKind::Any), FILE));
    }
}
