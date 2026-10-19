//! Reloads a document when its file changes on disk.

use std::{
    collections::HashMap,
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

/// One watcher per document window, by window label. Dropping a watcher
/// stops it.
#[derive(Default)]
pub struct Watchers(Mutex<HashMap<String, Debouncer<RecommendedWatcher, RecommendedCache>>>);

impl Watchers {
    /// Tells the window `label` to reload whenever `path` changes.
    ///
    /// The parent directory is watched rather than the file: many editors
    /// save by writing a new file and renaming it over the old one, which
    /// would end a watch on the file itself.
    pub fn watch(&self, app: &AppHandle, label: &str, path: &Path) {
        let Some(dir) = path.parent() else {
            return;
        };
        let (app, window, file) = (app.clone(), label.to_owned(), path.to_owned());

        let debouncer = new_debouncer(DEBOUNCE, None, move |result: DebounceEventResult| {
            if result.is_ok_and(|events| {
                events
                    .iter()
                    .any(|event| touches(event.kind, &event.paths, &file))
            }) {
                let _ = app.emit_to(&window, "document-changed", ());
            }
        })
        .and_then(|mut debouncer| {
            debouncer.watch(dir, RecursiveMode::NonRecursive)?;
            Ok(debouncer)
        });

        match debouncer {
            Ok(debouncer) => {
                self.lock().insert(label.to_owned(), debouncer);
            }
            Err(error) => eprintln!("mdr: can't watch {}: {error}", path.display()),
        }
    }

    pub fn stop(&self, label: &str) {
        self.lock().remove(label);
    }

    fn lock(
        &self,
    ) -> std::sync::MutexGuard<'_, HashMap<String, Debouncer<RecommendedWatcher, RecommendedCache>>>
    {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Whether an event in the watched directory concerns the document. Reads
/// don't count: they include mdr's own.
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
