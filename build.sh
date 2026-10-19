#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$SCRIPT_DIR"

if [ "$#" -gt 0 ] && [ "$1" = "--crosscompile" ]; then
    CROSS_COMPILE=1
    shift
fi

if [ "$#" -gt 1 ]; then
    printf '%s\n' "Usage: $0 [--crosscompile] [Rust target triple]" >&2
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

# A target the host does not run natively is a cross-compilation, which is
# opt-in: the caller asks for it with --crosscompile, or GitHub Actions asks
# for it by building a matrix of targets on runners built for one architecture
# (arm64 macOS runners also build x86_64, and 64-bit Windows runners also build
# 32-bit). Everything else builds for the machine it runs on.
if [ "$TARGET" != "$HOST" ] && [ "${CROSS_COMPILE:-0}" -ne 1 ] && [ "${GITHUB_ACTIONS:-}" != "true" ]; then
    printf 'Building %s on %s is cross-compilation. Pass --crosscompile to build it anyway.\n' \
        "$TARGET" "$HOST" >&2
    exit 2
fi

if [ "$PLATFORM" = linux ] && [ "$TARGET" != "$HOST" ] && [ "${GITHUB_ACTIONS:-}" = "true" ] && ! command -v cross >/dev/null 2>&1; then
    # GitHub Actions cross-compiles Linux through cross's containers, which the
    # workflow installs and which read the PipeWire development files for their
    # own architecture from Cross.toml. The runner has no cross toolchain of
    # its own, so falling back to cargo here would only produce a link error.
    # macOS and Windows reach their other architectures through the host
    # toolchain instead, so the guard does not apply to them.
    printf '%s\n' \
        "cross was not found. The release workflow installs it before non-native Linux targets are built; locally, install it with 'cargo install cross --locked'." >&2
    exit 1
fi

if [ "$TARGET" != "$HOST" ] && command -v cross >/dev/null 2>&1; then
    cross build --release --target "$TARGET"
else
    cargo build --release --target "$TARGET"
fi

BINARY="$SCRIPT_DIR/target/$TARGET/release/squiglab-legacy"
if [ "$PLATFORM" = windows ]; then
    BINARY="$BINARY.exe"
fi

cargo run --release --bin app-bundle -- package "$PLATFORM" "$BINARY" \
    "$SCRIPT_DIR/dist/$PLATFORM"
printf '\nBuild and package complete: %s\n' "$SCRIPT_DIR/dist/$PLATFORM"
