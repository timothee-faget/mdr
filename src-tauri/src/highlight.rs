//! Syntax highlighting of fenced code blocks, as CSS classes.
//!
//! The grammars and themes are bat's (two-face): syntect's own set lacks
//! TypeScript and TOML. Both are loaded on first use, so documents without
//! code don't pay for them. Regexes use Oniguruma: compiling a grammar is
//! 4 to 5 times faster than with fancy-regex (TypeScript: 14 ms vs 77 ms),
//! and the app is smaller.

use std::{borrow::Cow, collections::HashMap, fmt, sync::LazyLock};

use comrak::adapters::SyntaxHighlighterAdapter;
use syntect::{
    html::{ClassStyle, ClassedHTMLGenerator, css_for_theme_with_class_style},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};
use two_face::theme::LazyThemeSet;

/// Every class gets this prefix, so highlighting can't clash with the
/// page's own classes or those of raw HTML in the document.
const CLASSES: ClassStyle = ClassStyle::SpacedPrefixed { prefix: "hl-" };

static SYNTAXES: LazyLock<SyntaxSet> = LazyLock::new(two_face::syntax::extra_newlines);
static THEMES: LazyLock<LazyThemeSet> = LazyLock::new(|| two_face::theme::extra().into());

/// Highlights code blocks whose language is known, leaves the others plain.
pub struct Highlighter;

impl SyntaxHighlighterAdapter for Highlighter {
    fn write_highlighted(
        &self,
        output: &mut dyn fmt::Write,
        lang: Option<&str>,
        code: &str,
    ) -> fmt::Result {
        let Some(syntax) = lang.and_then(|lang| SYNTAXES.find_syntax_by_token(lang)) else {
            return comrak::html::escape(output, code);
        };

        let mut html = ClassedHTMLGenerator::new_with_class_style(syntax, &SYNTAXES, CLASSES);
        for line in LinesWithEndings::from(code) {
            if html
                .parse_html_for_line_which_includes_newline(line)
                .is_err()
            {
                // A grammar that fails mid-way: better plain than half-done.
                return comrak::html::escape(output, code);
            }
        }
        output.write_str(&html.finalize())
    }

    fn write_pre_tag(
        &self,
        output: &mut dyn fmt::Write,
        attributes: HashMap<&'static str, Cow<'_, str>>,
    ) -> fmt::Result {
        comrak::html::write_opening_tag(output, "pre", attributes)
    }

    fn write_code_tag(
        &self,
        output: &mut dyn fmt::Write,
        attributes: HashMap<&'static str, Cow<'_, str>>,
    ) -> fmt::Result {
        comrak::html::write_opening_tag(output, "code", attributes)
    }
}

/// Whether `name` is one of the bundled code themes.
pub fn has_theme(name: &str) -> bool {
    THEMES.theme_names().any(|known| known == name)
}

/// The token colors of a code theme, as CSS rules on the `hl-` classes.
/// The theme's background goes to `.hl-code`, a class the generator never
/// emits: code blocks keep the page theme's background.
pub fn theme_css(name: &str) -> String {
    THEMES
        .get(name)
        .and_then(|theme| css_for_theme_with_class_style(theme, CLASSES).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn highlight(lang: Option<&str>, code: &str) -> String {
        let mut html = String::new();
        Highlighter
            .write_highlighted(&mut html, lang, code)
            .unwrap();
        html
    }

    #[test]
    fn required_languages_are_highlighted() {
        for (lang, code) in [
            ("rust", "fn main() {}\n"),
            ("ts", "const x: number = 1;\n"),
            ("typescript", "let y = 2;\n"),
            ("python", "def f():\n    pass\n"),
            ("sh", "echo hi\n"),
            ("bash", "ls -la\n"),
            ("json", "{\"a\": 1}\n"),
            ("yaml", "a: 1\n"),
            ("toml", "a = 1\n"),
            ("diff", "-old\n+new\n"),
        ] {
            assert!(
                highlight(Some(lang), code).contains("hl-"),
                "{lang} isn't highlighted"
            );
        }
    }

    #[test]
    fn unknown_or_missing_languages_stay_plain_and_escaped() {
        for lang in [None, Some("no-such-language")] {
            assert_eq!(highlight(lang, "a < b\n"), "a &lt; b\n");
        }
    }

    #[test]
    fn default_code_themes_exist() {
        for name in ["InspiredGitHub", "Solarized (light)", "OneHalfDark"] {
            assert!(has_theme(name), "{name}");
            assert!(theme_css(name).contains(".hl-"), "{name}");
        }
    }
}
