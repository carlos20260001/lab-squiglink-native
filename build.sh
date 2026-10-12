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

HOST=$(rustc -vV | sed -n 's/^host: //p')
TARGET=${1:-$HOST}
case "$HOST" in
    *-linux-*) HOST_PLATFORM=linux ;;
    *-windows-*) HOST_PLATFORM=windows ;;
    *-apple-darwin) HOST_PLATFORM=macos ;;
    *) printf 'Unsupported build host: %s\n' "$HOST" >&2; exit 2 ;;
esac
case "$TARGET" in
    *-linux-*) PLATFORM=linux ;;
    *-windows-*) PLATFORM=windows ;;
    *-apple-darwin) PLATFORM=macos ;;
    *) printf 'Unsupported packaging target: %s\n' "$TARGET" >&2; exit 2 ;;
esac

if [ "$PLATFORM" != "$HOST_PLATFORM" ]; then
    printf 'Packaging %s applications requires a %s host (current host: %s).\n' \
        "$PLATFORM" "$PLATFORM" "$HOST_PLATFORM" >&2
    exit 1
fi

if [ "$TARGET" != "$HOST" ] && command -v cross >/dev/null 2>&1; then
    cross build --release --target "$TARGET"
else
    cargo build --release --target "$TARGET"
fi

BINARY="$SCRIPT_DIR/target/$TARGET/release/crigraph-native"
if [ "$PLATFORM" = windows ]; then
    BINARY="$BINARY.exe"
fi

cargo run --release --bin app-bundle -- package "$PLATFORM" "$BINARY" \
    "$SCRIPT_DIR/dist/$PLATFORM"
printf '\nBuild and package complete: %s\n' "$SCRIPT_DIR/dist/$PLATFORM"
