//! Markdown to sanitized HTML.

use std::sync::LazyLock;

use comrak::{Options, markdown_to_html};

/// Allow-list applied to comrak's output. Raw HTML is common in READMEs
/// (`<details>`, `<kbd>`, `<img>`), so it is parsed, then anything able to
/// run code or load remote frames is dropped here.
static SANITIZER: LazyLock<ammonia::Builder<'static>> = LazyLock::new(|| {
    let mut builder = ammonia::Builder::default();
    builder
        .add_tags(["section", "input"])
        .add_generic_attributes(["id", "class", "align"])
        .add_tag_attributes("input", ["type", "checked"])
        // Task list checkboxes are display-only.
        .set_tag_attribute_value("input", "disabled", "");
    builder
});

/// Renders GitHub Flavored Markdown to HTML that is safe to inject in the page.
#[must_use]
pub fn to_html(markdown: &str) -> String {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.tasklist = true;
    options.extension.autolink = true;
    options.extension.footnotes = true;
    options.extension.header_id_prefix = Some(String::new());
    options.extension.front_matter_delimiter = Some("---".to_owned());
    options.render.r#unsafe = true;

    SANITIZER
        .clean(&markdown_to_html(markdown, &options))
        .to_string()
}
