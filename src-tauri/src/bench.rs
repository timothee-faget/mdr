//! Startup probe for `scripts/bench.sh`, off unless `MDR_BENCH` is set.
//!
//! With `MDR_BENCH=1`, each page reports its first painted frame and mdr
//! prints `mdr: painted` on stderr. With `MDR_BENCH=quit`, it also quits,
//! so that launches can be timed one after the other.

use tauri::AppHandle;

pub fn enabled() -> bool {
    std::env::var_os("MDR_BENCH").is_some()
}

/// Tells ui/main.js to report its first painted frame.
pub const INIT_SCRIPT: &str = "window.__MDR_BENCH__ = true;";

/// Called by the page once its first frame with the document is painted.
// Tauri commands receive their arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn bench_painted(app: AppHandle) {
    eprintln!("mdr: painted");
    if std::env::var("MDR_BENCH").is_ok_and(|mode| mode == "quit") {
        app.exit(0);
    }
}
