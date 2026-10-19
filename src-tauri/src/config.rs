//! User configuration: `~/.config/mdr/config.toml`, turned into the CSS
//! variables used by `ui/style.css`.
//!
//! A broken config never stops the app: invalid values fall back to their
//! default with a warning on stderr.

use std::{
    collections::HashMap,
    fmt::Write as _,
    fs, io,
    path::PathBuf,
    sync::{Arc, Mutex, PoisonError},
};

use serde::Deserialize;
use tauri::{Theme, window::Color};

/// The config file as written by the user. Every key is optional.
#[derive(Deserialize, Default)]
#[serde(default)]
struct File {
    appearance: Appearance,
    light_theme: Option<String>,
    dark_theme: Option<String>,
    font: Font,
    layout: Layout,
    themes: HashMap<String, Colors<Option<String>>>,
}

#[derive(Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum Appearance {
    #[default]
    Auto,
    Light,
    Dark,
}

#[derive(Deserialize)]
#[serde(default)]
struct Font {
    family: String,
    mono: String,
    size: f32,
    line_height: f32,
}

impl Default for Font {
    fn default() -> Self {
        Self {
            family: "-apple-system, system-ui, sans-serif".to_owned(),
            mono: "ui-monospace, SF Mono, Menlo, monospace".to_owned(),
            size: 16.0,
            line_height: 1.6,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
struct Layout {
    max_width: String,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            max_width: "70ch".to_owned(),
        }
    }
}

/// A theme's colors: `Option<String>` as written, `Rgba` once resolved.
#[derive(Deserialize, Default, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
struct Colors<T> {
    background: T,
    text: T,
    muted: T,
    accent: T,
    code_background: T,
    border: T,
}

const PAPER: Colors<Rgba> = Colors {
    background: Rgba(0xfb, 0xfa, 0xf7, 0xff),
    text: Rgba(0x1f, 0x1f, 0x1f, 0xff),
    muted: Rgba(0x6b, 0x6b, 0x6b, 0xff),
    accent: Rgba(0x1f, 0x5f, 0xbf, 0xff),
    code_background: Rgba(0xf0, 0xee, 0xe8, 0xff),
    border: Rgba(0xe2, 0xdf, 0xd7, 0xff),
};

const SEPIA: Colors<Rgba> = Colors {
    background: Rgba(0xf4, 0xec, 0xd8, 0xff),
    text: Rgba(0x3b, 0x2f, 0x1e, 0xff),
    muted: Rgba(0x75, 0x66, 0x4e, 0xff),
    accent: Rgba(0x9c, 0x4a, 0x1a, 0xff),
    code_background: Rgba(0xea, 0xdf, 0xc5, 0xff),
    border: Rgba(0xdc, 0xcf, 0xb1, 0xff),
};

const NIGHT: Colors<Rgba> = Colors {
    background: Rgba(0x1c, 0x1c, 0x1e, 0xff),
    text: Rgba(0xe4, 0xe4, 0xe6, 0xff),
    muted: Rgba(0x9b, 0x9b, 0xa1, 0xff),
    accent: Rgba(0x6e, 0xaa, 0xff, 0xff),
    code_background: Rgba(0x2a, 0x2a, 0x2d, 0xff),
    border: Rgba(0x3a, 0x3a, 0x3e, 0xff),
};

fn built_in(name: &str) -> Option<Colors<Rgba>> {
    match name {
        "paper" => Some(PAPER),
        "sepia" => Some(SEPIA),
        "night" => Some(NIGHT),
        _ => None,
    }
}

/// The resolved config, ready for the windows.
#[derive(Debug)]
pub struct Style {
    /// Forced window appearance, or `None` to follow the system.
    pub appearance: Option<Theme>,
    light: Colors<Rgba>,
    dark: Colors<Rgba>,
    css: String,
}

impl Style {
    /// Window background for the effective appearance, painted before the
    /// page loads so that no white flashes.
    pub fn background(&self, theme: Theme) -> Color {
        let Rgba(r, g, b, a) = match theme {
            Theme::Dark => self.dark.background,
            _ => self.light.background,
        };
        Color(r, g, b, a)
    }

    /// The CSS variables, as injected in the pages.
    pub fn css(&self) -> &str {
        &self.css
    }

    /// Script run before the page renders: defines the CSS variables.
    pub fn init_script(&self) -> String {
        let css = serde_json::to_string(&self.css).unwrap_or_default();
        format!(
            "const style = document.createElement('style');\
             style.id = 'config';\
             style.textContent = {css};\
             document.documentElement.append(style);"
        )
    }
}

/// The current style, replaced when the config file changes.
pub struct Config(Mutex<Arc<Style>>);

impl Config {
    /// Reads the user's config, printing a warning for anything ignored.
    pub fn load() -> Self {
        let source = read().unwrap_or_else(|error| {
            eprintln!("mdr: config: can't read it ({error}), using defaults");
            String::new()
        });
        let (style, warnings) = parse(&source);
        print(&warnings);
        Self(Mutex::new(Arc::new(style)))
    }

    pub fn current(&self) -> Arc<Style> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Re-reads the config after a change and returns the new style. A file
    /// that can't be read or parsed keeps the previous style, so that a typo
    /// while editing doesn't reset the look.
    pub fn reload(&self) -> Option<Arc<Style>> {
        let source = read()
            .inspect_err(|error| eprintln!("mdr: config: can't read it ({error}), unchanged"))
            .ok()?;
        let (style, warnings) = reparse(&source)
            .inspect_err(|warning| eprintln!("mdr: config: {warning}"))
            .ok()?;
        print(&warnings);

        let style = Arc::new(style);
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Arc::clone(&style);
        Some(style)
    }
}

/// The config file's content, empty when there is none.
fn read() -> io::Result<String> {
    match path().map(fs::read_to_string) {
        Some(Err(error)) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Some(result) => result,
        None => Ok(String::new()),
    }
}

fn print(warnings: &[String]) {
    for warning in warnings {
        eprintln!("mdr: config: {warning}");
    }
}

/// `$XDG_CONFIG_HOME/mdr/config.toml`, else `~/.config/mdr/config.toml`.
pub fn path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("mdr").join("config.toml"))
}

/// Resolves a config file's content at launch. Never fails: an invalid file
/// gives the defaults, other problems become warnings.
fn parse(source: &str) -> (Style, Vec<String>) {
    reparse(source).unwrap_or_else(|_| {
        let error = deserialize(source, &mut Vec::new())
            .err()
            .unwrap_or_default();
        let mut warnings = vec![format!("invalid file ({error}), using defaults")];
        (resolve(&File::default(), &mut warnings), warnings)
    })
}

/// Like `parse`, but an invalid file is an error instead of the defaults.
fn reparse(source: &str) -> Result<(Style, Vec<String>), String> {
    let mut warnings = Vec::new();
    let file = deserialize(source, &mut warnings)
        .map_err(|error| format!("invalid file ({error}), keeping the previous config"))?;
    Ok((resolve(&file, &mut warnings), warnings))
}

fn deserialize(source: &str, warnings: &mut Vec<String>) -> Result<File, String> {
    toml::Deserializer::parse(source)
        .and_then(|deserializer| {
            serde_ignored::deserialize(deserializer, |path| {
                warnings.push(format!("unknown key `{path}`, ignored"));
            })
        })
        .map_err(|error| error.message().trim().to_owned())
}

fn resolve(file: &File, warnings: &mut Vec<String>) -> Style {
    let light = theme(file, file.light_theme.as_deref(), "paper", warnings);
    let dark = theme(file, file.dark_theme.as_deref(), "night", warnings);
    let css = css(file, &light, &dark, warnings);
    let appearance = match file.appearance {
        Appearance::Auto => None,
        Appearance::Light => Some(Theme::Light),
        Appearance::Dark => Some(Theme::Dark),
    };

    Style {
        appearance,
        light,
        dark,
        css,
    }
}

/// A built-in theme, a `[themes.<name>]` table, or both: a table named after
/// a built-in theme overrides some of its colors. Custom themes start from
/// `paper`.
fn theme(
    file: &File,
    name: Option<&str>,
    default: &str,
    warnings: &mut Vec<String>,
) -> Colors<Rgba> {
    let mut name = name.unwrap_or(default);
    if built_in(name).is_none() && !file.themes.contains_key(name) {
        warnings.push(format!("unknown theme `{name}`, using `{default}`"));
        name = default;
    }

    let base = built_in(name).unwrap_or(PAPER);
    let Some(custom) = file.themes.get(name) else {
        return base;
    };

    let mut color = |key: &str, value: &Option<String>, fallback: Rgba| {
        let Some(value) = value else {
            return fallback;
        };
        Rgba::parse(value).unwrap_or_else(|| {
            warnings.push(format!(
                "`themes.{name}.{key}`: `{value}` is not a color (use #rrggbb or rgb()), ignored"
            ));
            fallback
        })
    };

    Colors {
        background: color("background", &custom.background, base.background),
        text: color("text", &custom.text, base.text),
        muted: color("muted", &custom.muted, base.muted),
        accent: color("accent", &custom.accent, base.accent),
        code_background: color(
            "code_background",
            &custom.code_background,
            base.code_background,
        ),
        border: color("border", &custom.border, base.border),
    }
}

fn css(
    file: &File,
    light: &Colors<Rgba>,
    dark: &Colors<Rgba>,
    warnings: &mut Vec<String>,
) -> String {
    let defaults = (Font::default(), Layout::default());

    let family =
        font_list(&file.font.family, "font.family", warnings).unwrap_or(&defaults.0.family);
    let mono = font_list(&file.font.mono, "font.mono", warnings).unwrap_or(&defaults.0.mono);
    let size =
        in_range(file.font.size, 8.0..=48.0, "font.size", warnings).unwrap_or(defaults.0.size);
    let line_height = in_range(
        file.font.line_height,
        1.0..=3.0,
        "font.line_height",
        warnings,
    )
    .unwrap_or(defaults.0.line_height);
    let width = length(&file.layout.max_width, "layout.max_width", warnings)
        .unwrap_or(&defaults.1.max_width);

    let mut css = format!(
        ":root {{\n  color-scheme: light dark;\n  --font: {family};\n  --mono: {mono};\n  \
         --size: {size}px;\n  --line-height: {line_height};\n  --width: {width};\n"
    );
    write_colors(&mut css, light, "  ");
    css.push_str("}\n@media (prefers-color-scheme: dark) {\n  :root {\n");
    write_colors(&mut css, dark, "    ");
    css.push_str("  }\n}\n");
    css
}

fn write_colors(css: &mut String, colors: &Colors<Rgba>, indent: &str) {
    for (variable, color) in [
        ("--bg", colors.background),
        ("--fg", colors.text),
        ("--muted", colors.muted),
        ("--accent", colors.accent),
        ("--code-bg", colors.code_background),
        ("--border", colors.border),
    ] {
        let _ = writeln!(css, "{indent}{variable}: {color};");
    }
}

/// A CSS font-family list, without anything that could escape the rule.
fn font_list<'a>(value: &'a str, key: &str, warnings: &mut Vec<String>) -> Option<&'a str> {
    let valid = !value.trim().is_empty()
        && value
            .chars()
            .all(|c| c.is_alphanumeric() || " ,-_.'\"".contains(c));
    if !valid {
        warnings.push(format!("`{key}`: `{value}` is not a font list, ignored"));
    }
    valid.then_some(value)
}

fn in_range(
    value: f32,
    range: std::ops::RangeInclusive<f32>,
    key: &str,
    warnings: &mut Vec<String>,
) -> Option<f32> {
    let valid = range.contains(&value);
    if !valid {
        warnings.push(format!(
            "`{key}`: {value} is outside {}..={}, ignored",
            range.start(),
            range.end()
        ));
    }
    valid.then_some(value)
}

/// A CSS length such as `70ch`, `720px` or `90%`.
fn length<'a>(value: &'a str, key: &str, warnings: &mut Vec<String>) -> Option<&'a str> {
    let number = ["ch", "px", "em", "rem", "%"]
        .iter()
        .find_map(|unit| value.strip_suffix(unit));
    let valid = number.is_some_and(|n| n.parse::<f32>().is_ok_and(|n| n > 0.0));
    if !valid {
        warnings.push(format!(
            "`{key}`: `{value}` is not a length (e.g. 70ch, 720px), ignored"
        ));
    }
    valid.then_some(value)
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
struct Rgba(u8, u8, u8, u8);

impl Rgba {
    /// `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, `rgb(r, g, b)` and
    /// `rgba(r, g, b, a)`, commas or spaces, alpha as 0–1 or a percentage.
    fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        if let Some(hex) = value.strip_prefix('#') {
            return Self::from_hex(hex);
        }

        let inner = value
            .strip_prefix("rgba(")
            .or_else(|| value.strip_prefix("rgb("))?
            .strip_suffix(')')?;
        let parts: Vec<&str> = inner
            .split([',', ' ', '/'])
            .filter(|part| !part.is_empty())
            .collect();
        let channel = |part: &str| part.parse::<u8>().ok();
        let alpha = match parts.get(3) {
            None => 0xff,
            Some(part) => {
                let alpha = match part.strip_suffix('%') {
                    Some(percent) => percent.parse::<f32>().ok()? / 100.0,
                    None => part.parse::<f32>().ok()?,
                };
                if !(0.0..=1.0).contains(&alpha) {
                    return None;
                }
                // In 0..=255 by the check above.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let alpha = (alpha * 255.0).round() as u8;
                alpha
            }
        };
        if !(3..=4).contains(&parts.len()) {
            return None;
        }
        Some(Self(
            channel(parts[0])?,
            channel(parts[1])?,
            channel(parts[2])?,
            alpha,
        ))
    }

    fn from_hex(hex: &str) -> Option<Self> {
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let digit = |i: usize| u8::from_str_radix(&hex[i..=i], 16).ok().map(|d| d * 17);
        let pair = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
        match hex.len() {
            3 => Some(Self(digit(0)?, digit(1)?, digit(2)?, 0xff)),
            4 => Some(Self(digit(0)?, digit(1)?, digit(2)?, digit(3)?)),
            6 => Some(Self(pair(0)?, pair(2)?, pair(4)?, 0xff)),
            8 => Some(Self(pair(0)?, pair(2)?, pair(4)?, pair(6)?)),
            _ => None,
        }
    }
}

impl std::fmt::Display for Rgba {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(red, green, blue, alpha) = *self;
        write!(formatter, "#{red:02x}{green:02x}{blue:02x}")?;
        if alpha != 0xff {
            write!(formatter, "{alpha:02x}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The CSS of a config that must produce no warning.
    pub(crate) fn css_of(source: &str) -> String {
        let (style, warnings) = parse(source);
        assert!(warnings.is_empty(), "{warnings:?}");
        style.css
    }

    #[test]
    fn empty_file_gives_defaults_without_warnings() {
        let (style, warnings) = parse("");

        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(style.appearance, None);
        assert_eq!(style.light, PAPER);
        assert_eq!(style.dark, NIGHT);
    }

    #[test]
    fn default_css() {
        insta::assert_snapshot!(parse("").0.css);
    }

    #[test]
    fn full_example_css() {
        let (style, warnings) = parse(
            r##"
            appearance = "light"
            light_theme = "mine"
            dark_theme = "night"

            [font]
            family = "Iowan Old Style, Georgia, serif"
            mono = "JetBrains Mono, monospace"
            size = 18
            line_height = 1.7

            [layout]
            max_width = "680px"

            [themes.mine]
            background = "#f7f3ea"
            accent = "rgb(156, 74, 26)"
            "##,
        );

        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(style.appearance, Some(Theme::Light));
        insta::assert_snapshot!(style.css);
    }

    #[test]
    fn built_in_themes_can_be_selected() {
        let (style, _) = parse(r#"light_theme = "sepia""#);

        assert_eq!(style.light, SEPIA);
    }

    #[test]
    fn custom_theme_falls_back_to_paper_for_unset_colors() {
        let (style, warnings) = parse(
            r##"
            light_theme = "mine"
            [themes.mine]
            background = "#000"
            "##,
        );

        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(style.light.background, Rgba(0, 0, 0, 0xff));
        assert_eq!(style.light.text, PAPER.text);
    }

    #[test]
    fn built_in_theme_can_be_partially_overridden() {
        let (style, _) = parse(
            r##"
            [themes.night]
            accent = "#ff8800"
            "##,
        );

        assert_eq!(style.dark.accent, Rgba(0xff, 0x88, 0x00, 0xff));
        assert_eq!(style.dark.background, NIGHT.background);
    }

    #[test]
    fn invalid_color_falls_back_with_a_warning() {
        let (style, warnings) = parse(
            r#"
            [themes.paper]
            text = "red; } body { display: none"
            "#,
        );

        assert_eq!(style.light.text, PAPER.text);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("themes.paper.text"), "{warnings:?}");
    }

    #[test]
    fn unknown_keys_are_warnings() {
        let (style, warnings) = parse(
            r#"
            colour = "blue"
            [font]
            weight = 400
            "#,
        );

        assert_eq!(style.light, PAPER);
        assert_eq!(
            warnings,
            [
                "unknown key `colour`, ignored",
                "unknown key `font.weight`, ignored"
            ]
        );
    }

    #[test]
    fn unknown_theme_falls_back_with_a_warning() {
        let (style, warnings) = parse(r#"dark_theme = "dracula""#);

        assert_eq!(style.dark, NIGHT);
        assert_eq!(warnings, ["unknown theme `dracula`, using `night`"]);
    }

    #[test]
    fn invalid_toml_gives_defaults_with_one_warning() {
        let (style, warnings) = parse("appearance = ");

        assert_eq!(style.light, PAPER);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].starts_with("invalid file"), "{warnings:?}");
    }

    #[test]
    fn wrong_types_give_defaults_with_one_warning() {
        let (style, warnings) = parse(r#"appearance = "sometimes""#);

        assert_eq!(style.appearance, None);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
    }

    #[test]
    fn out_of_range_and_unsafe_values_are_ignored() {
        let (style, warnings) = parse(
            r#"
            [font]
            family = "x; } * { color: red"
            size = 200
            [layout]
            max_width = "calc(100% - 1px)"
            "#,
        );

        assert_eq!(warnings.len(), 3, "{warnings:?}");
        assert_eq!(style.css, parse("").0.css);
    }

    #[test]
    fn reload_keeps_the_previous_config_on_an_invalid_file() {
        let error = reparse("light_theme = \"sepia").unwrap_err();

        assert!(error.contains("keeping the previous config"), "{error}");
    }

    #[test]
    fn reload_accepts_what_launch_accepts() {
        let source = r#"light_theme = "sepia""#;
        let (reloaded, warnings) = reparse(source).unwrap();

        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(reloaded.css, parse(source).0.css);
    }

    #[test]
    fn colors_parse() {
        for (input, expected) in [
            ("#abc", Some(Rgba(0xaa, 0xbb, 0xcc, 0xff))),
            ("#abcd", Some(Rgba(0xaa, 0xbb, 0xcc, 0xdd))),
            ("#a1b2c3", Some(Rgba(0xa1, 0xb2, 0xc3, 0xff))),
            ("#a1b2c380", Some(Rgba(0xa1, 0xb2, 0xc3, 0x80))),
            ("rgb(1, 2, 3)", Some(Rgba(1, 2, 3, 0xff))),
            ("rgb(1 2 3 / 50%)", Some(Rgba(1, 2, 3, 0x80))),
            ("rgba(1, 2, 3, 0)", Some(Rgba(1, 2, 3, 0))),
            ("#ab", None),
            ("#gggggg", None),
            ("rgb(256, 0, 0)", None),
            ("rgb(1, 2)", None),
            ("rgba(1, 2, 3, 2)", None),
            ("red", None),
        ] {
            assert_eq!(Rgba::parse(input), expected, "{input}");
        }
    }

    #[test]
    fn colors_print_as_hex() {
        assert_eq!(Rgba(0xaa, 0xbb, 0xcc, 0xff).to_string(), "#aabbcc");
        assert_eq!(Rgba(0xaa, 0xbb, 0xcc, 0x80).to_string(), "#aabbcc80");
    }
}
