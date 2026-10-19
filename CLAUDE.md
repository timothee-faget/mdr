# mdr

Minimal read-only Markdown viewer for macOS. Rust + Tauri 2, front end in `ui/` (vanilla HTML/CSS/JS, no npm).

## Commands
- `just check`: fmt, clippy `-D warnings`, tests. Must pass before any commit.
- `just dev`: run the app.

## Rules
- English for code, commits, issues, docs. Conventional Commits.
- Work from a GitHub issue, on a branch, merged through a PR (squash).
- Rendering logic stays in pure functions, covered by `insta` snapshot tests.
- Never add a Node/npm dependency or remote scripts (strict CSP).
- Keep the window chrome-free: no toolbars or sidebars.
