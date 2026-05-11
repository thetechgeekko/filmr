# Android Integration

This document explains how to build `libfilmr.so` for Android and how the
JNI interface is consumed by the
[unprocess](https://github.com/thetechgeekko/unprocess) camera app.

## What is the Android integration?

`libfilmr.so` is a JNI shared library cross-compiled from this Rust crate
using [`cargo-ndk`](https://github.com/bbqsrc/cargo-ndk). It exposes the
filmr film-simulation and DNG-decode pipeline to Android via a small set of
JNI entry points declared in `src/android.rs`.

The Kotlin counterpart (`FilmrEngine.kt` in unprocess) loads the library at
runtime with `System.loadLibrary("filmr")` and calls the JNI functions through
`external fun` declarations.

## Prerequisites

| Tool | How to install |
|------|----------------|
| Rust stable toolchain | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| cargo-ndk 3.x | `cargo install cargo-ndk` |
| Android NDK r25c or r26 | Android Studio → SDK Manager → SDK Tools → NDK, or download from developer.android.com |
| `ANDROID_NDK_HOME` env var | Set to the NDK root, e.g. `~/Library/Android/sdk/ndk/25.2.9519653` |

### Add Rust Android targets

```bash
rustup target add \
    aarch64-linux-android \
    armv7-linux-androideabi \
    x86_64-linux-android \
    i686-linux-android
```

## Build commands

All commands are run from the root of this (`filmr`) repository. The build
script outputs `.so` files directly into `../unprocess/app/src/main/jniLibs/`,
so filmr and unprocess must be cloned as siblings.

```bash
# Release build — film simulation only (recommended for distribution)
./android/build-android.sh

# Release build — film simulation + Depth Anything V2 DOF/motion blur
./android/build-android.sh --with-depth

# Debug build — faster to compile, slower at runtime
./android/build-android.sh --debug
```

### Output location

```
../unprocess/app/src/main/jniLibs/
  arm64-v8a/libfilmr.so      # 64-bit ARM (modern Android phones)
  armeabi-v7a/libfilmr.so    # 32-bit ARM (older devices)
  x86_64/libfilmr.so         # 64-bit x86 (emulator)
  x86/libfilmr.so            # 32-bit x86 (emulator)
```

## Feature flags

| Cargo feature | Enabled by | Effect |
|---------------|------------|--------|
| `android` | Always on for Android builds | Compiles `src/android.rs` JNI entry points and the TIFF/DNG decoder |
| `depth` | `--with-depth` flag | Adds Depth Anything V2 monocular depth estimation (~16 MB extra per ABI) |

## JNI function reference

All functions are in the class `com.reilandeubank.unprocess.engine.FilmrEngine`.

### `processImage`

Apply film simulation to a decoded bitmap. No depth estimation.

```kotlin
private external fun processImage(
    rgbaBytes: ByteArray,  // ARGB_8888 pixels (R at offset 0)
    width: Int,
    height: Int,
    presetKey: String,     // e.g. "KODAK_PORTRA_400"
    styleKey: String,      // "ACCURATE" | "ARTISTIC" | "VINTAGE" | …
    configJson: String     // JSON-encoded SimulationConfig
): ByteArray?              // width×height×3 RGB bytes, or null on failure
```

### `processImageWithDepth`

Apply film simulation with depth-aware DOF and object-motion blur.
Depth estimation only runs when the `depth` feature is compiled in,
`modelPath` is non-empty, and `configJson` has `dof_amount > 0` or
`object_motion_amount > 0`.

```kotlin
private external fun processImageWithDepth(
    rgbaBytes: ByteArray,
    width: Int,
    height: Int,
    presetKey: String,
    styleKey: String,
    configJson: String,
    modelPath: String      // absolute path to depth_anything_v2_vits.rten, or ""
): ByteArray?
```

### `processRawDng`

Decode a raw DNG file (Malvar-He-Cutler demosaic + ColorMatrix1 colour
correction) then apply film simulation. Optionally uses depth estimation
when `modelPath` is provided.

```kotlin
private external fun processRawDng(
    dngBytes: ByteArray,   // raw DNG file contents
    presetKey: String,
    styleKey: String,
    configJson: String,
    modelPath: String      // absolute path to depth model, or ""
): ByteArray?              // [width: i32 LE][height: i32 LE][RGB bytes…]
```

Response layout:
- Bytes 0–3: image width as little-endian `i32`
- Bytes 4–7: image height as little-endian `i32`
- Bytes 8…: `width × height × 3` processed RGB bytes

### `isDepthSupported`

Returns `true` when the library was compiled with the `depth` feature.

```kotlin
private external fun isDepthSupported(): Boolean
```

### `getAvailablePresets`

Returns a JSON array of all built-in film presets.

```kotlin
private external fun getAvailablePresets(): String
// [{"manufacturer":"Kodak","name":"Portra 400","iso":400}, …]
```

### `getDefaultConfig`

Returns a JSON-encoded default `SimulationConfig`.

```kotlin
private external fun getDefaultConfig(): String
```

## DNG decode pipeline

The `processRawDng` function internally calls `decode_dng_to_rgb()` in
`src/android.rs`, which:

1. Reads TIFF metadata: Compression, BitsPerSample, CFAPattern (default RGGB),
   BlackLevel, WhiteLevel, ColorMatrix1.
2. Checks that Compression == 1 (uncompressed); rejects JPEG/lossless-JPEG/deflate
   with a clear error message.
3. Normalises all samples to `[0.0, 1.0]` using `(sample − black) / (white − black)`.
4. Runs a three-pass Malvar-He-Cutler gradient-corrected demosaic:
   - Pass 1: Green — 5-tap gradient-corrected formula at R/B sites.
   - Pass 2: Red — `(R − G)` colour-difference bilinear interpolation.
   - Pass 3: Blue — symmetric to Red.
5. Applies the ColorMatrix1-derived `camera → sRGB` 3×3 matrix (Bradford-adapted
   XYZ D50 → sRGB) when the tag is present.
