# mdr

A minimal, read-only Markdown viewer for macOS. One file per window, a solid background, no chrome: just the text.

- Native and light: Rust + [Tauri 2](https://tauri.app), rendered by the system WebView (no bundled browser)
- GitHub Flavored Markdown via [comrak](https://github.com/kivikakk/comrak)
- Configurable colors and typography (`~/.config/mdr/config.toml` or the Settings window)
- Reloads automatically when the file changes

> Status: early development. See the [milestones](https://github.com/timothee-faget/mdr/milestones).

## Install

Download the `.dmg` from the [latest release](https://github.com/timothee-faget/mdr/releases/latest).
Builds are not signed: the first time, right-click `mdr.app` → **Open**.

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE)
