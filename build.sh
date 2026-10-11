#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$SCRIPT_DIR"

if ! command -v cargo >/dev/null 2>&1; then
    printf '%s\n' "Cargo was not found. Install Rust from https://www.rust-lang.org/tools/install." >&2
    exit 1
fi

cargo build --release
printf '\nBuild complete: %s\n' "$SCRIPT_DIR/target/release/crigraph-native"
