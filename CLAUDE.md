# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build
cargo build
cargo build --features android
cargo build --all-features

# Test
cargo test -p filmr --no-default-features        # core tests only (fast)
cargo test                                        # all tests in workspace
cargo test test_name                              # single test by name
cargo test -p filmr -- tests::test_white_balance  # fully-qualified test path

# Lint / Format
cargo fmt --check
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo clippy --features android --all-targets -- -D warnings
cargo audit                                       # security audit (CI gate)

# Feature checks (CI gates)
cargo check --features android
cargo check --features android,depth

# Android cross-compile (requires cargo-ndk + NDK)
cd android && ./build-android.sh
# Targets: aarch64-linux-android, armv7-linux-androideabi, x86_64-linux-android, i686-linux-android
```

## Feature Flags

| Flag | Effect |
|------|--------|
| `android` | JNI entry points + TIFF/DNG decode (`src/android.rs`) |
| `depth` | Depth-of-field + object-motion blur via rten neural net (`src/depth.rs`) |
| `gpu` / `compute-gpu` | WGPU GPU acceleration for linearization, halation, MTF |
| `android,depth` | Full Android build with depth estimation |

No features enabled = pure Rust library with CPU simulation only.

## Architecture

### Entry Points
- **Library API**: `src/processor.rs` — `Processor::new(config)` + `process(image)` or `process_with_depth(image, model_path)`
- **Android JNI**: `src/android.rs` — `Java_com_reilandeubank_unprocess_engine_FilmrEngine_processImage`, `…processRawDng`, `…processImageWithDepth`
- **Config**: `SimulationConfig` (JSON-serializable) drives the entire pipeline

### Pipeline Flow

```
sRGB u8 image
    ↓ linearize (LUT)
    ↓ [optional pre-develop stages, applied in order]:
    │   MicroMotionStage     — physiological tremor (8-12 Hz, Selwyn dwell-weight)
    │   ObjectMotionStage    — depth-modulated motion blur
    │   DepthOfFieldStage    — mipmap bokeh + Petzval swirl
    │   RotationalBlurStage  — camera rotation around optical axis
    │   MtfStage             — optical softness (resolution_lp_mm → Gaussian)
    │   ChromaticAberrationStage — lateral RGB magnification difference
    ↓
  ┌─ Fast mode  ─────── DevelopStage (3x3 spectral matrix → exposure → H-D curve → density)
  └─ Accurate mode ──── AccurateDevelopStage (full Beer-Lambert propagation through FilmLayerStack)
    ↓
    saturation adjustment → vignette → auto-levels (1st/99th percentile)
    ↓ grain (Selwyn σ(D) = √(α·D + σ_read²), applied in density + output space)
    ↓ gamma encode (linear → sRGB 2.4)
    ↓ sRGB u8 output
```

### Key Source Files

| File | Purpose |
|------|---------|
| `src/processor.rs` | `SimulationConfig`, pipeline orchestration, `Processor` struct |
| `src/pipeline.rs` | `PipelineStage` trait, all stage structs, `create_linear_image`, `DevelopStage`, `AccurateDevelopStage` |
| `src/film.rs` | `FilmStock` (H-D curves, spectral params, grain, halation, color matrix), `with_style()` |
| `src/physics.rs` | `SegmentedCurve::map_erf()` / `map_smooth()` — H-D to density mapping |
| `src/spectral.rs` | `Spectrum` — 81 wavelength bins (380–780 nm, 5 nm), D65, blackbody |
| `src/spectral_engine.rs` | Beer-Lambert + Fresnel propagation through `FilmLayerStack` (Accurate mode) |
| `src/film_layer.rs` | `FilmLayerStack` physical structure (emulsion layers, dye spectra, inhibition matrix) |
| `src/grain.rs` | Selwyn grain model, spatially-correlated noise, shadow (shot) noise |
| `src/light_leak.rs` | Halation (threshold + Gaussian + additive tint), light leak shapes |
| `src/shake.rs` | `ShakeTrajectory` — physiological tremor synthesis |
| `src/utils.rs` | `apply_gaussian_blur` (3× box-blur, SIMD f32x4, separable) |
| `src/presets/` | 30+ calibrated film stocks (one file per manufacturer) |
| `src/android.rs` | JNI bridge: bounds checking (`check_jni_array_len`, `check_dng_dimensions`), JSON config deserialization, TIFF decode |
| `src/depth.rs` | rten model download, depth map inference, SHA-256 integrity plumbing |
| `src/metrics.rs` | `FilmMetrics` — per-channel stats, Lab, LBP, GLCM, PSD slope, entropy |

### Fast vs Accurate Mode

**Fast** (`SimulationMode::Fast`): A pre-computed 3×3 spectral matrix converts linear RGB to film-space exposure. One matrix multiply per pixel. Suitable for interactive preview.

**Accurate** (`SimulationMode::Accurate`): Every pixel is propagated through an 81-bin spectrum through a physical `FilmLayerStack` (forward + backward Beer-Lambert pass with Fresnel reflections at each interface). A normalization pass binary-searches for the scale factor that maps 18% gray to 18% output. ~20–50× slower than Fast but physically grounded.

### Adding a New Film Preset

1. Add a `FilmStock` constant to the appropriate file in `src/presets/` (copy the nearest stock as a template)
2. Register it in the manufacturer's `get_stocks()` vec in `src/presets/` and in the `stock_by_key()` match arm in `src/android.rs`
3. Run `cargo test` — the integration tests validate that all registered presets produce finite, non-clipping output

### JNI / Android Notes

- `check_jni_array_len(len: i32) -> Result<usize, String>` gates all `jint` → `usize` casts (MAX = 256 MB)
- `check_dng_dimensions(w, h: u32) -> Result<(), String>` caps DNG at 16 384 × 16 384
- JSON config errors propagate as `RuntimeException` (no `unwrap_or_default` on deserialisation)
- The `#[cfg(feature = "android")]` gate on JNI items is lifted to `#[cfg(any(feature = "android", test))]` in `lib.rs` so unit tests compile without a JVM
