#!/usr/bin/env bash
# Build filmr as an Android native shared library (.so) using cargo-ndk.
#
# Prerequisites
# -------------
# 1. Rust toolchain: https://rustup.rs/
# 2. cargo-ndk:    cargo install cargo-ndk
# 3. Android NDK:  export ANDROID_NDK_HOME=/path/to/ndk
# 4. Rust targets:
#      rustup target add \
#        aarch64-linux-android \
#        armv7-linux-androideabi \
#        x86_64-linux-android \
#        i686-linux-android
#
# Output
# ------
# Copies libfilmr.so into ../unprocess/app/src/main/jniLibs/<abi>/
# so the Android Gradle build picks them up automatically.
#
# Usage
# -----
#   ./android/build-android.sh            # release build
#   ./android/build-android.sh --debug    # debug build

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FILMR_ROOT="$(dirname "$SCRIPT_DIR")"
UNPROCESS_JNILIBS="${FILMR_ROOT}/../unprocess/app/src/main/jniLibs"

BUILD_TYPE="release"
if [[ "${1:-}" == "--debug" ]]; then
  BUILD_TYPE="debug"
fi

ABIS=(
  "arm64-v8a:aarch64-linux-android"
  "armeabi-v7a:armv7-linux-androideabi"
  "x86_64:x86_64-linux-android"
  "x86:i686-linux-android"
)

echo "==> Building filmr for Android (${BUILD_TYPE}) ..."

cd "$FILMR_ROOT"

for entry in "${ABIS[@]}"; do
  ABI="${entry%%:*}"
  TARGET="${entry##*:}"
  echo "    Building for ${ABI} (${TARGET}) ..."

  if [[ "$BUILD_TYPE" == "release" ]]; then
    cargo ndk -t "$ABI" build --release --features android
    SO_PATH="target/${TARGET}/release/libfilmr.so"
  else
    cargo ndk -t "$ABI" build --features android
    SO_PATH="target/${TARGET}/debug/libfilmr.so"
  fi

  DEST_DIR="${UNPROCESS_JNILIBS}/${ABI}"
  mkdir -p "$DEST_DIR"
  cp "$SO_PATH" "${DEST_DIR}/libfilmr.so"
  echo "    -> ${DEST_DIR}/libfilmr.so"
done

echo "==> Done. Libraries written to ${UNPROCESS_JNILIBS}"
