# List available recipes
default:
    @just --list

# Run the app with hot reload of the Rust code
dev *args:
    cargo tauri dev {{ args }}

# Everything CI checks: formatting, lints, tests
check: fmt-check clippy test

# Format all Rust code
fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace

# Audit dependencies (licenses, advisories, sources)
deny:
    cargo deny check

# Build the release .app and .dmg
build:
    cargo tauri build

# Build and copy mdr.app to /Applications
install: build
    rm -rf /Applications/mdr.app
    cp -R target/release/bundle/macos/mdr.app /Applications/

# Install the `mdr` command in ~/.local/bin
install-cli:
    install -d ~/.local/bin
    install -m 755 scripts/mdr ~/.local/bin/mdr

# Enable the repo's git hooks (run once after cloning)
hooks:
    git config core.hooksPath .githooks
