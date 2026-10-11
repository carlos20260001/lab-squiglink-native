#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$SCRIPT_DIR"

if [ "$#" -gt 1 ]; then
    printf '%s\n' "Usage: $0 [Rust target triple]" >&2
    exit 2
fi

if ! command -v cargo >/dev/null 2>&1; then
    printf '%s\n' "Cargo was not found. Install Rust from https://www.rust-lang.org/tools/install." >&2
    exit 1
fi

if [ "$#" -eq 1 ]; then
    TARGET=$1
    HOST=$(rustc -vV | sed -n 's/^host: //p')
    if [ "$TARGET" != "$HOST" ] && command -v cross >/dev/null 2>&1; then
        cross build --release --target "$TARGET"
    else
        cargo build --release --target "$TARGET"
    fi
    printf '\nBuild complete: %s\n' "$SCRIPT_DIR/target/$TARGET/release/crigraph-native"
else
    cargo build --release
    printf '\nBuild complete: %s\n' "$SCRIPT_DIR/target/release/crigraph-native"
fi
