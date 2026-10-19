# mdr

A minimal, read-only Markdown viewer for macOS. One file per window, a solid background, no chrome: just the text.

- Native and light: Rust + [Tauri 2](https://tauri.app), rendered by the system WebView (no bundled browser)
- GitHub Flavored Markdown via [comrak](https://github.com/kivikakk/comrak)
- Configurable colors and typography (`~/.config/mdr/config.toml` or the Settings window)
- Reloads automatically when the file changes

> Status: early development. See the [milestones](https://github.com/timothee-faget/mdr/milestones).

## Install

1. Download `mdr_<version>_universal.dmg` from the [latest release](https://github.com/timothee-faget/mdr/releases/latest) (Apple Silicon and Intel).
2. Open it and drag **mdr** to **Applications**.
3. mdr is not signed with an Apple Developer ID, so macOS blocks its first launch. Allow it once, either:
   - open mdr, dismiss the warning, then go to **System Settings › Privacy & Security** and click **Open Anyway** next to the message about mdr;
   - or run `xattr -dr com.apple.quarantine /Applications/mdr.app` in a terminal.

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE)
