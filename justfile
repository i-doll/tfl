default:
  @just --list

# Run clippy and tests
[group('dev')]
check: lint test

# Run clippy
[group('dev')]
lint:
  cargo clippy

# Run tests
[group('dev')]
test:
  cargo test

# Build release binary
[group('install')]
build:
  cargo build --release

# Install to cargo bin
[group('install')]
install:
  cargo install --path .

# Uninstall from cargo bin
[group('install')]
uninstall:
  cargo uninstall tfl

# Bump version, create release branch, commit, push, and open PR
[group('release')]
release version:
  #!/usr/bin/env bash
  set -euo pipefail
  if ! command -v cargo-set-version &> /dev/null; then
    echo "Installing cargo-edit for cargo set-version..."
    cargo install cargo-edit
  fi
  cargo set-version {{version}}
  cargo check
  git checkout -b "release/v{{version}}"
  git add Cargo.toml Cargo.lock
  git commit -m "chore(release): bump version to {{version}}"
  git push -u origin "release/v{{version}}"
  gh pr create \
    --title "release: v{{version}}" \
    --body "Bump version to {{version}}. Merging this PR will create tag \`v{{version}}\` and trigger a release build."
