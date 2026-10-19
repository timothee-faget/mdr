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

# Measure startup time, memory and size of a release build (opens mdr windows)
bench runs="10":
    cargo tauri build --bundles app
    scripts/bench.sh {{ runs }}

# Build the release .app and .dmg
build:
    cargo tauri build

# Build and copy mdr.app to /Applications
install: build
    rm -rf /Applications/mdr.app
    cp -R target/release/bundle/macos/mdr.app /Applications/

# Regenerate the app icons from assets/icon.svg
icon:
    swift scripts/render-icon.swift assets/icon.svg assets/icon.png
    cargo tauri icon assets/icon.png -o src-tauri/icons
    cd src-tauri/icons && rm -rf android ios Square*.png StoreLogo.png icon.ico 64x64.png

# Install the `mdr` command in ~/.local/bin
install-cli:
    install -d ~/.local/bin
    install -m 755 scripts/mdr ~/.local/bin/mdr

# Enable the repo's git hooks (run once after cloning)
hooks:
    git config core.hooksPath .githooks
