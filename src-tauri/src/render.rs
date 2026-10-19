//! Markdown to sanitized HTML.

use std::{
    borrow::Cow,
    path::{Component, Path, PathBuf},
};

use comrak::{Options, markdown_to_html_with_plugins, options::Plugins};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};

use crate::highlight::Highlighter;

/// What `encodeURIComponent` leaves alone, as Tauri's `convertFileSrc` does.
const URI_COMPONENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'!')
    .remove(b'~')
    .remove(b'*')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')');

/// Renders GitHub Flavored Markdown to HTML that is safe to inject in the page.
/// Relative image paths are resolved against `base`, the document's directory.
#[must_use]
pub fn to_html(markdown: &str, base: &Path) -> String {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.tasklist = true;
    options.extension.autolink = true;
    options.extension.footnotes = true;
    options.extension.header_id_prefix = Some(String::new());
    options.extension.front_matter_delimiter = Some("---".to_owned());
    options.render.r#unsafe = true;

    let mut plugins = Plugins::default();
    plugins.render.codefence_syntax_highlighter = Some(&Highlighter);

    sanitizer(base)
        .clean(&markdown_to_html_with_plugins(markdown, &options, &plugins))
        .to_string()
}

/// The local file a URL points to, relative to `base`, or `None` for a URL
/// with a scheme (`https:`, `data:`…) or a bare `#anchor`.
pub(crate) fn local_path(base: &Path, url: &str) -> Option<PathBuf> {
    let url = url.trim();
    if url.starts_with('#') || url.starts_with("//") || has_scheme(url) {
        return None;
    }
    let path = url.split(['?', '#']).next().unwrap_or_default();
    if path.is_empty() {
        return None;
    }
    let path = percent_decode_str(path).decode_utf8_lossy();
    Some(normalize(&base.join(path.as_ref())))
}

/// Allow-list applied to comrak's output. Raw HTML is common in READMEs
/// (`<details>`, `<kbd>`, `<img>`), so it is parsed, then anything able to
/// run code or load remote frames is dropped here.
fn sanitizer(base: &Path) -> ammonia::Builder<'static> {
    let base = base.to_owned();
    let mut builder = ammonia::Builder::default();
    builder
        .add_tags(["section", "input"])
        .add_generic_attributes(["id", "class", "align"])
        .add_tag_attributes("input", ["type", "checked"])
        // Task list checkboxes are display-only.
        .set_tag_attribute_value("input", "disabled", "")
        // Only kept on images, see below.
        .add_url_schemes(["data"])
        .attribute_filter(
            move |element, attribute, value| match (element, attribute) {
                ("img", "src") => Some(image_source(&base, value)),
                (_, "href" | "src" | "cite") if has_scheme_named(value, "data") => None,
                _ => Some(Cow::Borrowed(value)),
            },
        );
    builder
}

/// Local images go through Tauri's asset protocol, the only way the page can
/// read files. Its scope is the document's directory (see `documents.rs`).
fn image_source<'u>(base: &Path, src: &'u str) -> Cow<'u, str> {
    match local_path(base, src) {
        Some(path) => Cow::Owned(format!(
            "asset://localhost/{}",
            utf8_percent_encode(&path.to_string_lossy(), URI_COMPONENT)
        )),
        None => Cow::Borrowed(src),
    }
}

fn has_scheme(url: &str) -> bool {
    url.split_once(':').is_some_and(|(scheme, _)| {
        scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
    })
}

fn has_scheme_named(url: &str, name: &str) -> bool {
    url.trim()
        .split_once(':')
        .is_some_and(|(scheme, _)| scheme.eq_ignore_ascii_case(name))
}

/// Resolves `.` and `..` without touching the file system, so that
/// `docs/../../secret.png` can't pass for a path inside `docs`.
fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                normalized.pop();
            }
            Component::CurDir => {}
            other => normalized.push(other),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src(value: &str) -> String {
        image_source(Path::new("/docs"), value).into_owned()
    }

    #[test]
    fn relative_images_go_through_the_asset_protocol() {
        assert_eq!(src("pic.png"), "asset://localhost/%2Fdocs%2Fpic.png");
        assert_eq!(
            src("./img/pic.png"),
            "asset://localhost/%2Fdocs%2Fimg%2Fpic.png"
        );
    }

    #[test]
    fn parent_directories_are_resolved() {
        assert_eq!(src("../pic.png"), "asset://localhost/%2Fpic.png");
        assert_eq!(src("img/../../../pic.png"), "asset://localhost/%2Fpic.png");
    }

    #[test]
    fn absolute_paths_are_kept() {
        assert_eq!(src("/srv/pic.png"), "asset://localhost/%2Fsrv%2Fpic.png");
    }

    #[test]
    fn queries_fragments_and_escapes_are_handled() {
        assert_eq!(
            src("my%20pic.png?raw=true#top"),
            "asset://localhost/%2Fdocs%2Fmy%20pic.png"
        );
        assert_eq!(
            src("été.png"),
            "asset://localhost/%2Fdocs%2F%C3%A9t%C3%A9.png"
        );
    }

    #[test]
    fn remote_and_inline_images_are_untouched() {
        for url in [
            "https://example.com/pic.png",
            "//example.com/pic.png",
            "data:image/png;base64,iVBORw0KGgo=",
        ] {
            assert_eq!(src(url), url);
        }
    }

    #[test]
    fn data_urls_only_survive_on_images() {
        let html = to_html(
            "![x](data:image/gif;base64,R0lGOD) [y](data:text/html,<b>)",
            Path::new("/docs"),
        );

        assert!(
            html.contains(r#"src="data:image/gif;base64,R0lGOD""#),
            "{html}"
        );
        assert!(!html.contains("data:text/html"), "{html}");
    }

    #[test]
    fn links_are_left_relative() {
        let html = to_html("[next](other.md#intro)", Path::new("/docs"));

        assert!(html.contains(r#"href="other.md#intro""#), "{html}");
    }

    #[test]
    fn local_paths() {
        let base = Path::new("/docs");
        assert_eq!(
            local_path(base, "a.md#x"),
            Some(PathBuf::from("/docs/a.md"))
        );
        assert_eq!(local_path(base, "../b.md"), Some(PathBuf::from("/b.md")));
        assert_eq!(local_path(base, "#x"), None);
        assert_eq!(local_path(base, "?q"), None);
        assert_eq!(local_path(base, "https://example.com/a.md"), None);
        assert_eq!(local_path(base, "mailto:me@example.com"), None);
    }
}
