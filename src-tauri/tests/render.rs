use std::{fs, time::Instant};

use mdr_lib::render::to_html;

#[test]
fn fixtures() {
    insta::glob!("fixtures/*.md", |path| {
        let markdown = fs::read_to_string(path).unwrap();
        insta::assert_snapshot!(to_html(&markdown));
    });
}

#[test]
fn strips_scripts_and_handlers() {
    let html = to_html(&fs::read_to_string("tests/fixtures/malicious.md").unwrap());

    for forbidden in [
        "<script",
        "onerror",
        "onclick",
        "javascript:",
        "<iframe",
        "<style",
        "<form",
        "<object",
        "style=",
    ] {
        assert!(
            !html.contains(forbidden),
            "{forbidden:?} survived in:\n{html}"
        );
    }
}

#[test]
fn task_list_checkboxes_are_disabled() {
    let html = to_html("- [x] done\n- [ ] todo\n");

    assert_eq!(html.matches("<input").count(), 2);
    assert_eq!(html.matches("disabled").count(), 2);
}

/// Run with `cargo test --release -- --ignored`.
#[test]
#[ignore = "timing check, only meaningful in release mode"]
fn renders_one_megabyte_under_150ms() {
    let chunk = fs::read_to_string("tests/fixtures/lists.md").unwrap()
        + &fs::read_to_string("tests/fixtures/tables.md").unwrap()
        + &fs::read_to_string("tests/fixtures/code.md").unwrap();
    let markdown = chunk.repeat(1_000_000 / chunk.len() + 1);

    let start = Instant::now();
    let html = to_html(&markdown);
    let elapsed = start.elapsed();

    assert_ne!(html, "");
    assert!(elapsed.as_millis() < 150, "rendered 1 MB in {elapsed:?}");
}
