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

use serde::{Deserialize, Serialize};
use tauri::{Theme, window::Color};

use crate::highlight;

/// The config file as written by the user. Every key is optional.
#[derive(Deserialize, Default)]
#[serde(default)]
struct File {
    appearance: Appearance,
    light_theme: Option<String>,
    dark_theme: Option<String>,
    font: Font,
    layout: Layout,
    themes: HashMap<String, ThemeTable>,
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

/// A `[themes.<name>]` table as written. Every key is optional.
#[derive(Deserialize, Default)]
#[serde(default)]
struct ThemeTable {
    background: Option<String>,
    text: Option<String>,
    muted: Option<String>,
    accent: Option<String>,
    code_background: Option<String>,
    border: Option<String>,
    /// One of the bundled syntax highlighting themes (`highlight.rs`).
    code_theme: Option<String>,
}

/// A resolved theme's colors.
#[derive(Clone, Copy, Debug, PartialEq)]
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

/// The code theme of each built-in theme, also the base of custom themes
/// (through `paper`).
fn default_code_theme(name: &str) -> &'static str {
    match name {
        "sepia" => "Solarized (light)",
        "night" => "OneHalfDark",
        _ => "InspiredGitHub",
    }
}

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
    /// The CSS variables.
    css: String,
    /// The syntax highlighting rules of the light and dark code themes.
    code_css: String,
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

    /// The CSS injected in the pages: variables and code colors.
    pub fn css(&self) -> String {
        format!("{}{}", self.css, self.code_css)
    }

    /// Script run before the page renders: defines the CSS variables.
    pub fn init_script(&self) -> String {
        let css = serde_json::to_string(&self.css()).unwrap_or_default();
        format!(
            "const style = document.createElement('style');\
             style.id = 'config';\
             style.textContent = {css};\
             document.documentElement.append(style);"
        )
    }
}

/// The config file created when there is none yet: every option, commented
/// out, so the file starts with the defaults.
pub const TEMPLATE: &str = r##"# mdr configuration. Changes apply as soon as the file is saved.
# Every key is optional: uncomment and edit.

# appearance = "auto"        # auto | light | dark
# light_theme = "paper"      # paper, sepia, or a [themes.<name>] below
# dark_theme = "night"       # night, or a [themes.<name>] below

# [font]
# family = "-apple-system, system-ui, sans-serif"   # a CSS font-family list
# mono = "ui-monospace, SF Mono, Menlo, monospace"
# size = 16
# line_height = 1.6

# [layout]
# max_width = "70ch"         # ch, px, em, rem or %

# [themes.mine]              # unset colors come from paper
# background = "#f7f3ea"     # #rrggbb or rgb()
# text = "#2b2b2b"
# muted = "#7a7468"
# accent = "#9c4a1a"
# code_background = "#efe9dc"
# border = "#e2dccd"
# code_theme = "InspiredGitHub"   # code colors, also for built-in themes:
#   InspiredGitHub, GitHub, Solarized (light), Solarized (dark), OneHalfLight,
#   OneHalfDark, Nord, Dracula, gruvbox-light, gruvbox-dark, Catppuccin Latte,
#   Catppuccin Mocha, Monokai Extended, TwoDark, zenburn…
"##;

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

/// The config file's path, creating it from the template if needed.
pub fn ensure_file() -> io::Result<PathBuf> {
    let path = path().ok_or_else(|| io::Error::other("no home directory"))?;
    if !path.exists() {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(&path, TEMPLATE)?;
    }
    Ok(path)
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
    let (light, light_code) = theme(file, file.light_theme.as_deref(), "paper", warnings);
    let (dark, dark_code) = theme(file, file.dark_theme.as_deref(), "night", warnings);
    let css = css(file, &light, &dark, warnings);
    // Each code theme in its own query: a token colored by only one of them
    // must not keep that color in the other appearance.
    let code_css = format!(
        "@media (prefers-color-scheme: light) {{\n{}}}\n\
         @media (prefers-color-scheme: dark) {{\n{}}}\n",
        highlight::theme_css(&light_code),
        highlight::theme_css(&dark_code),
    );
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
        code_css,
    }
}

/// The theme to use: `name` if it's built in or has a table, `default` if no
/// name is given, an error with the name if it's unknown.
fn theme_name<'a>(
    file: &File,
    name: Option<&'a str>,
    default: &'a str,
) -> Result<&'a str, &'a str> {
    let name = name.unwrap_or(default);
    if built_in(name).is_some() || file.themes.contains_key(name) {
        Ok(name)
    } else {
        Err(name)
    }
}

/// A built-in theme, a `[themes.<name>]` table, or both: a table named after
/// a built-in theme overrides some of its colors. Custom themes start from
/// `paper`. Returns the colors and the code theme.
fn theme(
    file: &File,
    name: Option<&str>,
    default: &str,
    warnings: &mut Vec<String>,
) -> (Colors<Rgba>, String) {
    let name = theme_name(file, name, default).unwrap_or_else(|unknown| {
        warnings.push(format!("unknown theme `{unknown}`, using `{default}`"));
        default
    });

    let base = built_in(name).unwrap_or(PAPER);
    let base_code = default_code_theme(name);
    let Some(custom) = file.themes.get(name) else {
        return (base, base_code.to_owned());
    };

    let code = match &custom.code_theme {
        Some(code) if highlight::has_theme(code) => code.clone(),
        Some(code) => {
            warnings.push(format!(
                "`themes.{name}.code_theme`: unknown code theme `{code}`, using `{base_code}`"
            ));
            base_code.to_owned()
        }
        None => base_code.to_owned(),
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

    let colors = Colors {
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
    };
    (colors, code)
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

/// The keys the Settings window may write.
pub fn is_editable(path: &[&str]) -> bool {
    const COLORS: [&str; 6] = [
        "background",
        "text",
        "muted",
        "accent",
        "code_background",
        "border",
    ];
    match path {
        [key] => ["appearance", "light_theme", "dark_theme"].contains(key),
        ["font", key] => ["family", "size", "line_height"].contains(key),
        ["layout", "max_width"] => true,
        ["themes", _, key] => *key == "code_theme" || COLORS.contains(key),
        _ => false,
    }
}

/// Sets one key of a config file's content (or removes it, with `None`),
/// leaving the rest as written: comments, order and formatting. Missing
/// tables are created.
pub fn edit(
    source: &str,
    path: &[&str],
    value: Option<toml_edit::Value>,
) -> Result<String, String> {
    let mut document: toml_edit::DocumentMut =
        source.parse().map_err(|error: toml_edit::TomlError| {
            format!("the config file isn't valid TOML ({})", error.message())
        })?;
    let (key, tables) = path.split_last().ok_or("empty key")?;

    // A file of comments only, like the template: toml_edit keeps them as the
    // document's tail and would print the first key above them.
    let comments = document
        .as_table()
        .is_empty()
        .then(|| document.trailing().as_str().unwrap_or_default().to_owned());
    if comments.is_some() {
        document.set_trailing("");
    }

    let mut table = document.as_table_mut();
    for (depth, name) in tables.iter().enumerate() {
        let item = table.entry(name).or_insert_with(|| {
            let mut new = toml_edit::Table::new();
            // `[themes.mine]`, not an empty `[themes]` header above it.
            new.set_implicit(depth + 1 < tables.len());
            toml_edit::Item::Table(new)
        });
        table = item
            .as_table_mut()
            .ok_or_else(|| format!("`{name}` isn't a table in the config file"))?;
    }

    match value {
        Some(mut value) => match table.get_mut(key).and_then(|item| item.as_value_mut()) {
            // Keeps a comment written after the old value.
            Some(old) => {
                *value.decor_mut() = old.decor().clone();
                *old = value;
            }
            None => {
                table.insert(key, toml_edit::Item::Value(value));
            }
        },
        None => {
            table.remove(key);
        }
    }

    if let Some(comments) = comments {
        put_first(document.as_table_mut(), comments);
    }
    Ok(document.to_string())
}

/// Prefixes the first line `table` prints with `text`. Implicit tables
/// print nothing themselves: their first child does.
fn put_first(table: &mut toml_edit::Table, text: String) {
    if let Some((mut key, item)) = table.iter_mut().next() {
        match item {
            toml_edit::Item::Table(child) if child.is_implicit() => put_first(child, text),
            toml_edit::Item::Table(child) => child.decor_mut().set_prefix(text),
            _ => key.leaf_decor_mut().set_prefix(text),
        }
    }
}

/// What the Settings window shows: the config as resolved, defaults
/// included.
#[derive(Serialize)]
pub struct Settings {
    appearance: &'static str,
    light: ThemeSettings,
    dark: ThemeSettings,
    /// Built-in themes, then the `[themes.<name>]` tables, resolved for the
    /// theme cards.
    themes: Vec<ThemeSettings>,
    code_themes: Vec<&'static str>,
    font_family: String,
    font_size: f32,
    max_width: String,
}

#[derive(Serialize)]
struct ThemeSettings {
    name: String,
    /// `#rrggbb`, as `<input type="color">` expects, by config key.
    colors: HashMap<&'static str, String>,
    code_theme: String,
}

/// The current config file, resolved for the Settings window.
pub fn settings() -> Settings {
    let source = read().unwrap_or_default();
    let file = deserialize(&source, &mut Vec::new()).unwrap_or_default();
    let ignored = &mut Vec::new();

    let theme_settings = |name: Option<&str>, default: &str| {
        let name = theme_name(&file, name, default).unwrap_or(default);
        let (colors, code_theme) = theme(&file, Some(name), default, &mut Vec::new());
        let hex = |Rgba(red, green, blue, _): Rgba| format!("#{red:02x}{green:02x}{blue:02x}");
        ThemeSettings {
            name: name.to_owned(),
            colors: HashMap::from([
                ("background", hex(colors.background)),
                ("text", hex(colors.text)),
                ("muted", hex(colors.muted)),
                ("accent", hex(colors.accent)),
                ("code_background", hex(colors.code_background)),
                ("border", hex(colors.border)),
            ]),
            code_theme,
        }
    };

    let mut custom: Vec<String> = file
        .themes
        .keys()
        .filter(|name| built_in(name).is_none())
        .cloned()
        .collect();
    custom.sort();
    let defaults = (Font::default(), Layout::default());

    Settings {
        appearance: match file.appearance {
            Appearance::Auto => "auto",
            Appearance::Light => "light",
            Appearance::Dark => "dark",
        },
        light: theme_settings(file.light_theme.as_deref(), "paper"),
        dark: theme_settings(file.dark_theme.as_deref(), "night"),
        themes: ["paper", "sepia", "night"]
            .map(str::to_owned)
            .into_iter()
            .chain(custom)
            .map(|name| theme_settings(Some(&name), "paper"))
            .collect(),
        code_themes: highlight::theme_names(),
        font_family: font_list(&file.font.family, "", ignored)
            .unwrap_or(&defaults.0.family)
            .to_owned(),
        font_size: in_range(file.font.size, 8.0..=48.0, "", ignored).unwrap_or(defaults.0.size),
        max_width: length(&file.layout.max_width, "", ignored)
            .unwrap_or(&defaults.1.max_width)
            .to_owned(),
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
    fn code_themes_follow_the_page_themes() {
        let (style, _) = parse(r#"light_theme = "sepia""#);

        assert!(
            style
                .code_css
                .contains(&highlight::theme_css("Solarized (light)"))
        );
        assert!(
            style
                .code_css
                .contains(&highlight::theme_css("OneHalfDark"))
        );
    }

    #[test]
    fn code_theme_can_be_chosen_per_theme() {
        let (style, warnings) = parse(
            r#"
            [themes.night]
            code_theme = "Dracula"
            "#,
        );

        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(style.code_css.contains(&highlight::theme_css("Dracula")));
    }

    #[test]
    fn unknown_code_theme_falls_back_with_a_warning() {
        let (style, warnings) = parse(
            r#"
            [themes.paper]
            code_theme = "Nope"
            "#,
        );

        assert_eq!(
            warnings,
            ["`themes.paper.code_theme`: unknown code theme `Nope`, using `InspiredGitHub`"]
        );
        assert!(
            style
                .code_css
                .contains(&highlight::theme_css("InspiredGitHub"))
        );
    }

    fn edited(source: &str, path: &[&str], value: impl Into<toml_edit::Value>) -> String {
        edit(source, path, Some(value.into())).unwrap()
    }

    #[test]
    fn edit_keeps_comments_order_and_formatting() {
        let source = "# My config\nappearance = \"auto\"  # follows macOS\n\n[font]\n# Reading font\nsize = 16\nfamily = \"Charter\"\n";

        let output = edited(source, &["font", "size"], 18);

        assert_eq!(
            output,
            "# My config\nappearance = \"auto\"  # follows macOS\n\n[font]\n# Reading font\nsize = 18\nfamily = \"Charter\"\n"
        );
    }

    #[test]
    fn edit_keeps_the_comment_after_a_replaced_value() {
        let output = edited(
            "appearance = \"auto\"  # follows macOS\n",
            &["appearance"],
            "dark",
        );

        assert_eq!(output, "appearance = \"dark\"  # follows macOS\n");
    }

    #[test]
    fn edit_creates_missing_tables() {
        let output = edited(
            "appearance = \"dark\"\n",
            &["themes", "my theme", "accent"],
            "#ff8800",
        );

        assert_eq!(
            output,
            "appearance = \"dark\"\n\n[themes.\"my theme\"]\naccent = \"#ff8800\"\n"
        );
        assert_eq!(parse(&output).1, Vec::<String>::new());
    }

    #[test]
    fn edit_removes_keys() {
        let output = edit(
            "[themes.paper]\naccent = \"#ff0000\"\ntext = \"#000000\"\n",
            &["themes", "paper", "accent"],
            None,
        )
        .unwrap();

        assert_eq!(output, "[themes.paper]\ntext = \"#000000\"\n");
    }

    #[test]
    fn edit_on_the_template_gives_a_valid_config() {
        let output = edited(TEMPLATE, &["light_theme"], "sepia");

        assert!(output.starts_with("# mdr configuration."), "{output}");
        let (style, warnings) = parse(&output);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(style.light, SEPIA);
    }

    #[test]
    fn edit_on_the_template_keeps_its_comments_above_a_new_table() {
        let output = edited(TEMPLATE, &["themes", "paper", "accent"], "#ff8800");

        assert!(output.starts_with(TEMPLATE), "{output}");
        assert!(
            output.ends_with("[themes.paper]\naccent = \"#ff8800\"\n"),
            "{output}"
        );
    }

    #[test]
    fn edit_refuses_invalid_toml() {
        assert!(edit("appearance = ", &["appearance"], Some("dark".into())).is_err());
    }

    #[test]
    fn only_schema_keys_are_editable() {
        assert!(is_editable(&["appearance"]));
        assert!(is_editable(&["font", "size"]));
        assert!(is_editable(&["themes", "anything", "accent"]));
        assert!(is_editable(&["themes", "paper", "code_theme"]));
        assert!(!is_editable(&["colour"]));
        assert!(!is_editable(&["font", "weight"]));
        assert!(!is_editable(&["themes", "paper"]));
        assert!(!is_editable(&[]));
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
