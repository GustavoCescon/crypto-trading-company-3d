#!/bin/sh
set -eu
rustup component add rustfmt clippy
cargo fmt --all
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace -- --include-ignored
