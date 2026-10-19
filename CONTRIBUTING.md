# Contributing

## Setup

Requirements: macOS 13+, the Xcode Command Line Tools, [rustup](https://rustup.rs).

```sh
brew install just
cargo install tauri-cli cargo-deny --locked
just hooks   # enable the pre-commit hook
just dev     # run the app
```

## Commands

| Command | What it does |
|---|---|
| `just dev` | Run the app in development mode |
| `just check` | Format check, clippy (`-D warnings`), tests: same as CI |
| `just fmt` | Format the code |
| `just deny` | Audit dependencies |
| `just build` | Build `mdr.app` and the `.dmg` |
| `just install` | Build and copy to `/Applications` |
| `just install-cli` | Install the `mdr` command in `~/.local/bin` |

## Workflow

1. Every change starts from an issue.
2. Branch from `main`: `feat/12-settings-window`, `fix/31-table-overflow`…
3. Open a pull request that says `Closes #N`. CI must be green.
4. PR titles follow [Conventional Commits](https://www.conventionalcommits.org): `feat: …`, `fix: …`, `chore: …`, `docs: …`, `ci: …`, `refactor: …`, `test: …`. PRs are squash-merged, so the title becomes the commit on `main`.
5. Releases are automated by [release-please](https://github.com/googleapis/release-please): it keeps a release PR open with the next version and changelog. Merging it tags the release and CI attaches the universal `.dmg`.

## Layout

```
src-tauri/   Rust app (Tauri): rendering, config, file watching, menus
ui/          The WebView page: plain HTML/CSS/JS, no build step
scripts/     The `mdr` shell command
```

## Principles

- Read-only viewer. No editing features.
- Nothing on screen but the document.
- Fast startup and low memory before new features.
- No Node toolchain: the front end stays vanilla.
