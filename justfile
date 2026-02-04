default:
  @just --list

build:
  cargo build --release

install:
  cargo install --path .

uninstall:
  cargo uninstall tfl

test:
  cargo test

lint:
  cargo clippy

check: lint test

# Bump version, create release branch, commit, push, and open PR
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
