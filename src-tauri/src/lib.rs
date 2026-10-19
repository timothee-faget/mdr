mod documents;
pub mod render;

use std::path::PathBuf;

use tauri::{Manager, RunEvent, WindowEvent};

/// Starts the app.
///
/// # Panics
///
/// If the Tauri app can't be built, which only happens with an invalid
/// `tauri.conf.json`.
pub fn run() {
    let app = tauri::Builder::default()
        .manage(documents::Paths::default())
        .invoke_handler(tauri::generate_handler![documents::get_document])
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
        RunEvent::WindowEvent {
            label,
            event: WindowEvent::Destroyed,
            ..
        } => app.state::<documents::Paths>().remove(&label),
        _ => {}
    });
}

/// Files passed to the binary itself (`target/release/mdr file.md`).
fn cli_paths() -> Vec<PathBuf> {
    std::env::args_os()
        .skip(1)
        .filter(|arg| !arg.to_string_lossy().starts_with('-'))
        .map(PathBuf::from)
        .collect()
}
