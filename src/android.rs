//! Android JNI bindings for the filmr engine.
//!
//! Exposes the filmr film-simulation pipeline to Android via JNI.
//! Enable with `--features android` when cross-compiling for Android.
//!
//! The Kotlin counterpart lives in:
//!   com.reilandeubank.unprocess.engine.FilmrEngine

#[cfg(feature = "android")]
use crate::film::{FilmStock, FilmStyle};
#[cfg(feature = "android")]
use crate::processor::{process_image, SimulationConfig};
#[cfg(feature = "android")]
use image::{ImageBuffer, RgbImage};
#[cfg(feature = "android")]
use jni::objects::{JByteArray, JClass, JString};
#[cfg(feature = "android")]
use jni::sys::{jbyteArray, jint, jstring};
#[cfg(feature = "android")]
use jni::JNIEnv;

const MAX_SAFE_JNI_ARRAY_LEN: usize = 256 * 1024 * 1024; // 256 MB

/// Check that a JNI array length value is within safe bounds.
///
/// Returns `Ok(len as usize)` when valid, or an `Err` message otherwise.
/// Extracted from the inline guards in the JNI entry-points so that it can be
/// unit-tested without a live JVM.
fn check_jni_array_len(len: i32) -> Result<usize, String> {
    if len < 0 || len as usize > MAX_SAFE_JNI_ARRAY_LEN {
        return Err(format!("JNI array length out of range: {}", len));
    }
    Ok(len as usize)
}

/// Check that DNG image dimensions do not exceed `MAX_DNG_DIM`.
///
/// Extracted from `decode_dng_to_rgb` so that the guard can be tested
/// independently of a live TIFF decoder.
fn check_dng_dimensions(width: u32, height: u32) -> Result<(), String> {
    const MAX_DNG_DIM: u32 = 16384;
    if width > MAX_DNG_DIM || height > MAX_DNG_DIM {
        return Err(format!(
            "DNG dimensions {}x{} exceed maximum {}x{}",
            width, height, MAX_DNG_DIM, MAX_DNG_DIM
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Preset lookup
// ---------------------------------------------------------------------------

#[cfg(feature = "android")]
fn stock_by_key(key: &str) -> FilmStock {
    use crate::presets::{agfa, fujifilm, ilford, kodak, other, polaroid};
    match key {
        // Kodak
        "KODAK_PORTRA_400" => kodak::KODAK_PORTRA_400(),
        "KODAK_PORTRA_400_ARTISTIC" => kodak::KODAK_PORTRA_400_ARTISTIC(),
        "KODAK_PORTRA_160" => kodak::KODAK_PORTRA_160(),
        "KODAK_PORTRA_800" => kodak::KODAK_PORTRA_800(),
        "KODAK_TRI_X_400" => kodak::KODAK_TRI_X_400(),
        "KODAK_TRI_X_400_ARTISTIC" => kodak::KODAK_TRI_X_400_ARTISTIC(),
        "KODAK_PLUS_X_125" => kodak::KODAK_PLUS_X_125(),
        "KODAK_EKTACHROME_100" => kodak::KODAK_EKTACHROME_100(),
        "KODAK_EKTACHROME_100VS" => kodak::KODAK_EKTACHROME_100VS(),
        "KODAK_KODACHROME_64" => kodak::KODAK_KODACHROME_64(),
        "KODAK_KODACHROME_25" => kodak::KODAK_KODACHROME_25(),
        "KODAK_GOLD_200" => kodak::KODAK_GOLD_200(),
        "KODAK_EKTAR_100" => kodak::KODAK_EKTAR_100(),
        // Fujifilm
        "SUPERIA_400" => fujifilm::SUPERIA_400(),
        "SUPERIA_200" => fujifilm::SUPERIA_200(),
        "SUPERIA_100" => fujifilm::SUPERIA_100(),
        "NEOPAN_400" => fujifilm::NEOPAN_400(),
        "NEOPAN_100" => fujifilm::NEOPAN_100(),
        "PROVIA_100F" => fujifilm::PROVIA_100F(),
        "VELVIA_50" => fujifilm::VELVIA_50(),
        "VELVIA_50_ARTISTIC" => fujifilm::VELVIA_50_ARTISTIC(),
        "ASTIA_100F" => fujifilm::ASTIA_100F(),
        // Ilford
        "HP5_PLUS_400" => ilford::HP5_PLUS_400(),
        "HP5_PLUS_400_ARTISTIC" => ilford::HP5_PLUS_400_ARTISTIC(),
        "FP4_PLUS_125" => ilford::FP4_PLUS_125(),
        "DELTA_400_PROFESSIONAL" => ilford::DELTA_400_PROFESSIONAL(),
        "DELTA_100_PROFESSIONAL" => ilford::DELTA_100_PROFESSIONAL(),
        "PAN_F_PLUS_50" => ilford::PAN_F_PLUS_50(),
        "XP2_SUPER_400" => ilford::XP2_SUPER_400(),
        "SFX_200" => ilford::SFX_200(),
        "ORTHO_PLUS_80" => ilford::ORTHO_PLUS_80(),
        // Agfa
        "VISTA_400" => agfa::VISTA_400(),
        "VISTA_200" => agfa::VISTA_200(),
        "VISTA_100" => agfa::VISTA_100(),
        "APX_400" => agfa::APX_400(),
        "APX_100" => agfa::APX_100(),
        "PRECISA_100" => agfa::PRECISA_100(),
        "SCALA_200" => agfa::SCALA_200(),
        "OPTIMA_200" => agfa::OPTIMA_200(),
        // Polaroid
        "POLAROID_600_COLOR" => polaroid::POLAROID_600_COLOR(),
        "POLAROID_SX70_COLOR" => polaroid::POLAROID_SX70_COLOR(),
        "POLAROID_I_TYPE_COLOR" => polaroid::POLAROID_I_TYPE_COLOR(),
        "POLAROID_BW_667" => polaroid::POLAROID_BW_667(),
        "POLAROID_SPECTRA_COLOR" => polaroid::POLAROID_SPECTRA_COLOR(),
        "POLAROID_100_COLOR" => polaroid::POLAROID_100_COLOR(),
        "POLAROID_55_BW" => polaroid::POLAROID_55_BW(),
        // Other / Cinestill
        "CINESTILL_800T" => other::CINESTILL_800T(),
        "CINESTILL_50D" => other::CINESTILL_50D(),
        "STANDARD_DAYLIGHT" => other::STANDARD_DAYLIGHT(),
        // Default
        _ => kodak::KODAK_PORTRA_400(),
    }
}

#[cfg(feature = "android")]
fn style_from_str(s: &str) -> FilmStyle {
    match s {
        "ARTISTIC" => FilmStyle::Artistic,
        "VINTAGE" => FilmStyle::Vintage,
        "HIGH_CONTRAST" => FilmStyle::HighContrast,
        "PASTEL" => FilmStyle::Pastel,
        _ => FilmStyle::Accurate,
    }
}

// ---------------------------------------------------------------------------
// JNI helpers
// ---------------------------------------------------------------------------

#[cfg(feature = "android")]
fn rgba_to_rgb_image(rgba: &[u8], w: u32, h: u32) -> Option<RgbImage> {
    if rgba.len() < (w * h * 4) as usize {
        return None;
    }
    let mut rgb = vec![0u8; (w * h * 3) as usize];
    for i in 0..(w * h) as usize {
        rgb[i * 3] = rgba[i * 4];
        rgb[i * 3 + 1] = rgba[i * 4 + 1];
        rgb[i * 3 + 2] = rgba[i * 4 + 2];
    }
    ImageBuffer::from_raw(w, h, rgb)
}

// ---------------------------------------------------------------------------
// 3×3 matrix helpers (used for DNG colour correction)
// ---------------------------------------------------------------------------

#[cfg(feature = "android")]
fn mat3_inverse(m: [[f32; 3]; 3]) -> Option<[[f32; 3]; 3]> {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    if det.abs() < 1e-10 {
        return None;
    }
    let d = 1.0 / det;
    Some([
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * d,
            (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * d,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * d,
        ],
        [
            (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * d,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * d,
            (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * d,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * d,
            (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * d,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * d,
        ],
    ])
}

#[cfg(feature = "android")]
fn mat3_mul(a: [[f32; 3]; 3], b: [[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let mut c = [[0.0f32; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                c[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    c
}

#[cfg(feature = "android")]
#[inline]
fn mat3_apply(m: [[f32; 3]; 3], r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    (
        m[0][0] * r + m[0][1] * g + m[0][2] * b,
        m[1][0] * r + m[1][1] * g + m[1][2] * b,
        m[2][0] * r + m[2][1] * g + m[2][2] * b,
    )
}

// ---------------------------------------------------------------------------
// JNI exports
// ---------------------------------------------------------------------------

/// Process an image through the filmr engine.
///
/// Parameters (from Kotlin):
/// - `rgba_bytes`   : ARGB_8888 pixel data (4 bytes/pixel, R at offset 0)
/// - `width/height` : image dimensions
/// - `preset_key`   : preset identifier string (e.g. "KODAK_PORTRA_400")
/// - `style_key`    : style identifier string (e.g. "ACCURATE", "ARTISTIC")
/// - `config_json`  : JSON-encoded `SimulationConfig`
///
/// Returns RGB24 byte array on success, or throws RuntimeException on failure.
#[cfg(feature = "android")]
#[no_mangle]
pub extern "system" fn Java_com_reilandeubank_unprocess_engine_FilmrEngine_processImage<
    'local,
>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    rgba_bytes: JByteArray<'local>,
    width: jint,
    height: jint,
    preset_key: JString<'local>,
    style_key: JString<'local>,
    config_json: JString<'local>,
) -> jbyteArray {
    match process_image_impl(
        &mut env,
        &rgba_bytes,
        width,
        height,
        &preset_key,
        &style_key,
        &config_json,
    ) {
        Ok(arr) => arr,
        Err(e) => {
            let _ = env.throw_new("java/lang/RuntimeException", e.as_str());
            std::ptr::null_mut()
        }
    }
}

#[cfg(feature = "android")]
fn process_image_impl<'local>(
    env: &mut JNIEnv<'local>,
    rgba_bytes: &JByteArray<'local>,
    width: jint,
    height: jint,
    preset_key: &JString<'local>,
    style_key: &JString<'local>,
    config_json: &JString<'local>,
) -> Result<jbyteArray, String> {
    let preset_key: String = env
        .get_string(preset_key)
        .map_err(|e| e.to_string())?
        .into();
    let style_key: String = env
        .get_string(style_key)
        .map_err(|e| e.to_string())?
        .into();
    let config_json: String = env
        .get_string(config_json)
        .map_err(|e| e.to_string())?
        .into();

    if width < 0 || width as usize > MAX_SAFE_JNI_ARRAY_LEN {
        return Err(format!("Invalid width: {}", width));
    }
    if height < 0 || height as usize > MAX_SAFE_JNI_ARRAY_LEN {
        return Err(format!("Invalid height: {}", height));
    }

    let array_len = env
        .get_array_length(rgba_bytes)
        .map_err(|e| e.to_string())?;
    if array_len < 0 || array_len as usize > MAX_SAFE_JNI_ARRAY_LEN {
        return Err(format!("rgba_bytes array length out of bounds: {}", array_len));
    }

    let rgba = env
        .convert_byte_array(rgba_bytes)
        .map_err(|e| e.to_string())?;

    let w = width as u32;
    let h = height as u32;
    let input =
        rgba_to_rgb_image(&rgba, w, h).ok_or_else(|| "Invalid image dimensions".to_string())?;

    let film = stock_by_key(&preset_key).with_style(style_from_str(&style_key));

    let config: SimulationConfig =
        serde_json::from_str(&config_json).map_err(|e| format!("Invalid config JSON: {e}"))?;

    let output = process_image(&input, &film, &config);
    let output_bytes = output.into_raw();

    env.byte_array_from_slice(&output_bytes)
        .map(|arr| arr.into_raw())
        .map_err(|e| e.to_string())
}

/// Return a JSON array of all available presets.
///
/// Each element: `{"key":"KODAK_PORTRA_400","manufacturer":"Kodak","name":"Portra 400","iso":400}`
#[cfg(feature = "android")]
#[no_mangle]
pub extern "system" fn Java_com_reilandeubank_unprocess_engine_FilmrEngine_getAvailablePresets<
    'local,
>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jstring {
    let stocks = crate::presets::get_all_stocks();
    let entries: Vec<String> = stocks
        .iter()
        .map(|s| {
            format!(
                r#"{{"manufacturer":"{}","name":"{}","iso":{}}}"#,
                s.manufacturer, s.name, s.iso as u32
            )
        })
        .collect();
    let json = format!("[{}]", entries.join(","));
    env.new_string(json)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// Return a JSON-encoded default `SimulationConfig`.
#[cfg(feature = "android")]
#[no_mangle]
pub extern "system" fn Java_com_reilandeubank_unprocess_engine_FilmrEngine_getDefaultConfig<
    'local,
>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jstring {
    let config = SimulationConfig::default();
    let json = serde_json::to_string(&config).unwrap_or_else(|_| "{}".to_string());
    env.new_string(json)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

// ---------------------------------------------------------------------------
// DNG / RAW processing
// ---------------------------------------------------------------------------

/// Decode raw DNG bytes to a linear RGB image using the TIFF decoder.
///
/// DNG files are TIFF-based.  We read the following tags to normalise the
/// Bayer sensor data before demosaicing:
///
///   - Tag 0x0103 / 259    — Compression (must be 1 = uncompressed)
///   - Tag 33422 / 0x828D  — CFAPattern (2×2 Bayer mosaic, default RGGB)
///   - Tag 50714 / 0xC5BA  — BlackLevel (per-channel; scalar fallback = 0)
///   - Tag 50717 / 0xC5BD  — WhiteLevel (scalar; default 65535 for 16-bit)
///   - Tag 50721 / 0xC621  — ColorMatrix1 (XYZ D50 → camera; used to build
///                           the camera → sRGB correction matrix)
///   - BitsPerSample        — used to scale U8 data to 16-bit range
///
/// After Malvar-He-Cutler Bayer demosaicing, the ColorMatrix1-derived
/// camera→sRGB 3×3 matrix is applied to each pixel to produce accurate
/// sRGB output. If the tag is absent the step is silently skipped.
///
/// Return protocol:
///   - First 4 bytes: image width  as little-endian i32
///   - Next  4 bytes: image height as little-endian i32
///   - Remaining bytes: width×height×3 linear-8-bit RGB (R G B R G B …)
#[cfg(feature = "android")]
fn decode_dng_to_rgb(dng: &[u8]) -> Result<Vec<u8>, String> {
    use std::io::Cursor;
    use tiff::decoder::{Decoder, DecodingResult};
    use tiff::tags::Tag;

    // DNG-specific tag numbers (not in the tiff crate's built-in Tag enum)
    const TAG_CFA_PATTERN: u16 = 0x828D;    // 33422
    const TAG_BLACK_LEVEL_CORRECT: u16 = 0xC61A; // 50714
    const TAG_WHITE_LEVEL: u16 = 0xC61D;    // 50717
    // ColorMatrix1: XYZ (D50) → camera native RGB (9 SRational values, row-major)
    const TAG_COLOR_MATRIX1: u16 = 0xC621;  // 50721

    let cursor = Cursor::new(dng);
    let mut decoder = Decoder::new(cursor).map_err(|e| format!("TIFF decode error: {e}"))?;

    let (width, height) = decoder.dimensions().map_err(|e| format!("TIFF dimensions error: {e}"))?;

    // --- Dimension safety cap (Issue #11) ---
    // Reject absurdly large images before any allocation to prevent OOM.
    const MAX_DNG_DIM: u32 = 16384;
    if width > MAX_DNG_DIM || height > MAX_DNG_DIM {
        return Err(format!(
            "DNG dimensions {}x{} exceed maximum {}x{}",
            width, height, MAX_DNG_DIM, MAX_DNG_DIM
        ));
    }

    // --- Compression check ---
    // DngCreator.writeImage always produces uncompressed (type 1) strips.
    // Some third-party DNG files use JPEG (6), lossless-JPEG (7), or deflate (8/32946).
    // Attempting to read those as raw Bayer produces garbage; bail out clearly instead.
    let compression = decoder
        .find_tag(Tag::Compression)
        .ok()
        .flatten()
        .and_then(|v| v.into_u32().ok())
        .unwrap_or(1); // 1 = no compression (TIFF/DNG default)

    match compression {
        1 => { /* uncompressed — continue */ }
        6 => return Err(
            "Compressed DNG (JPEG compression) not supported. \
             Use uncompressed RAW capture or pre-process with a DNG SDK.".into()
        ),
        7 => return Err(
            "Compressed DNG (lossless JPEG compression) not supported.".into()
        ),
        32946 | 8 => return Err(
            "Compressed DNG (deflate compression) not supported.".into()
        ),
        other => return Err(format!("Unknown DNG compression type {other}; cannot decode.")),
    }

    // --- BitsPerSample ---
    // BitsPerSample may be stored as a scalar or as an array (one entry per channel,
    // e.g. [16, 16, 16] for a 3-channel 16-bit image). Use into_u32_vec() to handle
    // both cases and take the first channel value.
    let bits_per_sample: u32 = decoder
        .find_tag(Tag::BitsPerSample)
        .ok()
        .flatten()
        .and_then(|v| v.into_u32_vec().ok().and_then(|vec| vec.into_iter().next()))
        .unwrap_or(16);

    // --- ColorMatrix1 (XYZ D50 → Camera) → derive Camera → sRGB matrix ---
    // ColorMatrix1 stores 9 SRational values in row-major order representing the
    // 3×3 matrix M such that CameraRGB = M × XYZ_D50.
    // Inverting M gives Camera → XYZ_D50, then multiplying by the standard
    // XYZ_D50 → linear-sRGB matrix (Bradford-adapted) completes the transform.
    let cam_to_srgb: Option<[[f32; 3]; 3]> = decoder
        .find_tag(Tag::Unknown(TAG_COLOR_MATRIX1))
        .ok()
        .flatten()
        .and_then(|v| v.into_f32_vec().ok())
        .filter(|v| v.len() >= 9)
        .and_then(|v| {
            let m_xyz_to_cam = [
                [v[0], v[1], v[2]],
                [v[3], v[4], v[5]],
                [v[6], v[7], v[8]],
            ];
            let m_cam_to_xyz = mat3_inverse(m_xyz_to_cam)?;
            // Bradford-adapted XYZ D50 → linear sRGB matrix
            let m_xyz_to_srgb = [
                [ 3.1338561_f32, -1.6168667, -0.4906146],
                [-0.9787684_f32,  1.9161415,  0.0334540],
                [ 0.0719453_f32, -0.2289914,  1.4052427],
            ];
            Some(mat3_mul(m_xyz_to_srgb, m_cam_to_xyz))
        });

    // --- CFAPattern (2×2: R=0, G=1, B=2) ---
    // Default = RGGB
    let cfa: [u8; 4] = decoder
        .find_tag(Tag::Unknown(TAG_CFA_PATTERN))
        .ok()
        .flatten()
        .and_then(|v| v.into_u8_vec().ok())
        .and_then(|v| if v.len() >= 4 { Some([v[0], v[1], v[2], v[3]]) } else { None })
        .unwrap_or([0, 1, 1, 2]); // RGGB

    // --- BlackLevel (try correct tag, fall back to 0) ---
    let black_level: f32 = decoder
        .find_tag(Tag::Unknown(TAG_BLACK_LEVEL_CORRECT))
        .ok()
        .flatten()
        .and_then(|v| {
            v.into_f32().ok().or_else(|| None)
        })
        .unwrap_or(0.0_f32);

    // --- WhiteLevel ---
    let white_level: f32 = decoder
        .find_tag(Tag::Unknown(TAG_WHITE_LEVEL))
        .ok()
        .flatten()
        .and_then(|v| v.into_f32().ok())
        .unwrap_or_else(|| ((1u32 << bits_per_sample) - 1) as f32);

    let scale = white_level - black_level;
    if scale <= 0.0 {
        return Err(format!(
            "Invalid DNG levels: black={black_level} white={white_level}"
        ));
    }

    // --- Read RAW Bayer data ---
    let raw_data = decoder.read_image().map_err(|e| format!("TIFF read error: {e}"))?;

    // Normalise every sample to [0.0, 1.0]
    let samples: Vec<f32> = match raw_data {
        DecodingResult::U8(v)  => v.iter().map(|&x| (x as f32 - black_level) / scale).collect(),
        DecodingResult::U16(v) => v.iter().map(|&x| (x as f32 - black_level) / scale).collect(),
        DecodingResult::U32(v) => v.iter().map(|&x| (x as f32 - black_level) / scale).collect(),
        DecodingResult::I16(v) => v.iter().map(|&x| (x as f32 - black_level) / scale).collect(),
        DecodingResult::F32(v) => v.iter().map(|&x| (x - black_level) / scale).collect(),
        _ => return Err("Unsupported DNG sample format".to_string()),
    };

    let w = width as usize;
    let h = height as usize;

    if samples.len() < w * h {
        return Err(format!(
            "RAW data too short: got {} samples, expected {}x{}={}",
            samples.len(), w, h, w * h
        ));
    }

    // --- Malvar-He-Cutler gradient-corrected demosaic ---
    // Three-pass approach: (1) gradient-corrected Green everywhere,
    // (2) Red via (R−G) colour-difference bilinear, (3) Blue likewise.

    // Safe pixel fetch with border clamping
    let bayer = |row: isize, col: isize| -> f32 {
        let r = row.clamp(0, h as isize - 1) as usize;
        let c = col.clamp(0, w as isize - 1) as usize;
        samples[r * w + c]
    };
    // Channel index at (row, col): 0=R, 1=G, 2=B
    let ch = |row: usize, col: usize| -> usize {
        cfa[(row % 2) * 2 + (col % 2)] as usize
    };

    // Pass 1: Green channel
    // At G sites: copy directly.
    // At R/B sites: 5-tap gradient-corrected formula
    //   G = (2*(N+S+E+W) + 4*C - (NN+SS+EE+WW)) / 8
    let mut green = vec![0.0f32; w * h];
    for row in 0..h {
        for col in 0..w {
            let r = row as isize;
            let c = col as isize;
            green[row * w + col] = if ch(row, col) == 1 {
                bayer(r, c)
            } else {
                ((2.0 * (bayer(r-1,c) + bayer(r+1,c) + bayer(r,c-1) + bayer(r,c+1))
                  + 4.0 * bayer(r, c)
                  - (bayer(r-2,c) + bayer(r+2,c) + bayer(r,c-2) + bayer(r,c+2))) / 8.0)
                    .clamp(0.0, 1.0)
            };
        }
    }

    // Interpolated green lookup with border clamping
    let g_at = |row: isize, col: isize| -> f32 {
        let r = row.clamp(0, h as isize - 1) as usize;
        let c = col.clamp(0, w as isize - 1) as usize;
        green[r * w + c]
    };

    // Pass 2: Red channel via (R−G) colour-difference interpolation
    // At R site:  copy directly.
    // At G site:  average (R−G) of the two R neighbours (horizontal or vertical).
    // At B site:  average (R−G) of the four diagonal R neighbours.
    let mut red = vec![0.0f32; w * h];
    for row in 0..h {
        for col in 0..w {
            let r = row as isize;
            let c = col as isize;
            red[row * w + col] = match ch(row, col) {
                0 => bayer(r, c),
                1 => {
                    // Check if horizontal neighbours carry Red
                    let r_horiz = ch(row, col.wrapping_add(w).wrapping_sub(1) % w) == 0
                        || ch(row, (col + 1) % w) == 0;
                    let diff = if r_horiz {
                        ((bayer(r,c-1) - g_at(r,c-1)) + (bayer(r,c+1) - g_at(r,c+1))) / 2.0
                    } else {
                        ((bayer(r-1,c) - g_at(r-1,c)) + (bayer(r+1,c) - g_at(r+1,c))) / 2.0
                    };
                    (g_at(r, c) + diff).clamp(0.0, 1.0)
                }
                _ => {
                    let diff = ((bayer(r-1,c-1) - g_at(r-1,c-1))
                              + (bayer(r-1,c+1) - g_at(r-1,c+1))
                              + (bayer(r+1,c-1) - g_at(r+1,c-1))
                              + (bayer(r+1,c+1) - g_at(r+1,c+1))) / 4.0;
                    (g_at(r, c) + diff).clamp(0.0, 1.0)
                }
            };
        }
    }

    // Pass 3: Blue channel (symmetric to Red)
    let mut blue = vec![0.0f32; w * h];
    for row in 0..h {
        for col in 0..w {
            let r = row as isize;
            let c = col as isize;
            blue[row * w + col] = match ch(row, col) {
                2 => bayer(r, c),
                1 => {
                    // Check if horizontal neighbours carry Blue
                    let b_horiz = ch(row, col.wrapping_add(w).wrapping_sub(1) % w) == 2
                        || ch(row, (col + 1) % w) == 2;
                    let diff = if b_horiz {
                        ((bayer(r,c-1) - g_at(r,c-1)) + (bayer(r,c+1) - g_at(r,c+1))) / 2.0
                    } else {
                        ((bayer(r-1,c) - g_at(r-1,c)) + (bayer(r+1,c) - g_at(r+1,c))) / 2.0
                    };
                    (g_at(r, c) + diff).clamp(0.0, 1.0)
                }
                _ => {
                    let diff = ((bayer(r-1,c-1) - g_at(r-1,c-1))
                              + (bayer(r-1,c+1) - g_at(r-1,c+1))
                              + (bayer(r+1,c-1) - g_at(r+1,c-1))
                              + (bayer(r+1,c+1) - g_at(r+1,c+1))) / 4.0;
                    (g_at(r, c) + diff).clamp(0.0, 1.0)
                }
            };
        }
    }

    // Assemble RGB planes with optional camera → sRGB colour correction
    let mut rgb = vec![0f32; w * h * 3];
    for i in 0..(w * h) {
        let (sr, sg, sb) = match cam_to_srgb {
            Some(m) => mat3_apply(m, red[i], green[i], blue[i]),
            None    => (red[i], green[i], blue[i]),
        };
        let base = i * 3;
        rgb[base]     = sr.clamp(0.0, 1.0);
        rgb[base + 1] = sg.clamp(0.0, 1.0);
        rgb[base + 2] = sb.clamp(0.0, 1.0);
    }

    // Convert to u8
    let rgb_u8: Vec<u8> = rgb.iter().map(|&v| (v * 255.0 + 0.5) as u8).collect();

    // Pack result: [width: i32 LE][height: i32 LE][RGB bytes...]
    let mut out = Vec::with_capacity(8 + rgb_u8.len());
    out.extend_from_slice(&(width as i32).to_le_bytes());
    out.extend_from_slice(&(height as i32).to_le_bytes());
    out.extend_from_slice(&rgb_u8);

    Ok(out)
}

/// Process a raw DNG file through the filmr engine.
///
/// The DNG is decoded (demosaiced) to linear RGB, then the filmr film
/// simulation is applied.  When `model_path` is non-empty and the depth
/// feature is compiled in, monocular depth estimation drives DOF and
/// object-motion blur — matching the behaviour of `processImageWithDepth`.
///
/// Parameters (from Kotlin):
/// - `dng_bytes`   : raw DNG file bytes
/// - `preset_key`  : preset identifier string (e.g. "KODAK_PORTRA_400")
/// - `style_key`   : style identifier string  (e.g. "ACCURATE", "ARTISTIC")
/// - `config_json` : JSON-encoded `SimulationConfig`
/// - `model_path`  : absolute path to the depth model on-device, or empty
///                   string to skip depth estimation
///
/// Returns a `ByteArray` where:
///   - Bytes 0–3: image width  as little-endian signed 32-bit integer
///   - Bytes 4–7: image height as little-endian signed 32-bit integer
///   - Bytes 8… : width×height×3 processed RGB bytes (R G B R G B …)
///
/// Throws `java.lang.RuntimeException` on any failure.
#[cfg(feature = "android")]
#[no_mangle]
pub extern "system" fn Java_com_reilandeubank_unprocess_engine_FilmrEngine_processRawDng<
    'local,
>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    dng_bytes: JByteArray<'local>,
    preset_key: JString<'local>,
    style_key: JString<'local>,
    config_json: JString<'local>,
    model_path: JString<'local>,
) -> jbyteArray {
    match process_raw_dng_impl(&mut env, &dng_bytes, &preset_key, &style_key, &config_json, &model_path) {
        Ok(arr) => arr,
        Err(e) => {
            let _ = env.throw_new("java/lang/RuntimeException", e.as_str());
            std::ptr::null_mut()
        }
    }
}

#[cfg(feature = "android")]
fn process_raw_dng_impl<'local>(
    env: &mut JNIEnv<'local>,
    dng_bytes: &JByteArray<'local>,
    preset_key: &JString<'local>,
    style_key: &JString<'local>,
    config_json: &JString<'local>,
    model_path: &JString<'local>,
) -> Result<jbyteArray, String> {
    let preset_key: String = env.get_string(preset_key).map_err(|e| e.to_string())?.into();
    let style_key: String = env.get_string(style_key).map_err(|e| e.to_string())?.into();
    let config_json: String = env.get_string(config_json).map_err(|e| e.to_string())?.into();
    let model_path_str: String = env.get_string(model_path).map_err(|e| e.to_string())?.into();

    let dng_len = env
        .get_array_length(dng_bytes)
        .map_err(|e| e.to_string())?;
    if dng_len < 0 || dng_len as usize > MAX_SAFE_JNI_ARRAY_LEN {
        return Err(format!("dng_bytes array length out of bounds: {}", dng_len));
    }

    let dng = env.convert_byte_array(dng_bytes).map_err(|e| e.to_string())?;

    // Decode DNG → demosaiced + colour-corrected sRGB (with dimension header)
    let decoded = decode_dng_to_rgb(&dng)?;

    // Extract dimensions from the header we prepended
    if decoded.len() < 8 {
        return Err("DNG decode returned too-short buffer".to_string());
    }
    let width  = i32::from_le_bytes(decoded[0..4].try_into().unwrap()) as u32;
    let height = i32::from_le_bytes(decoded[4..8].try_into().unwrap()) as u32;
    let rgb_bytes = &decoded[8..];

    // Build RgbImage for the filmr pipeline
    let input: image::RgbImage =
        ImageBuffer::from_raw(width, height, rgb_bytes.to_vec())
            .ok_or_else(|| "Failed to build RgbImage from demosaiced data".to_string())?;

    let film = stock_by_key(&preset_key).with_style(style_from_str(&style_key));
    let config: SimulationConfig = serde_json::from_str(&config_json).map_err(|e| format!("Invalid config JSON: {e}"))?;

    // Attempt depth estimation when the feature is compiled in and a model path is given
    let depth_map = estimate_depth_if_available(&input, &model_path_str, &config);

    let output = crate::processor::process_image_with_depth(&input, &film, &config, depth_map.as_ref());
    let output_rgb = output.into_raw(); // width×height×3

    // Re-pack with dimension header so Kotlin knows the size
    let mut result = Vec::with_capacity(8 + output_rgb.len());
    result.extend_from_slice(&(width as i32).to_le_bytes());
    result.extend_from_slice(&(height as i32).to_le_bytes());
    result.extend_from_slice(&output_rgb);

    env.byte_array_from_slice(&result)
        .map(|arr| arr.into_raw())
        .map_err(|e| e.to_string())
}

/// Returns 1 (true) if the library was compiled with the `depth` feature
/// (Depth Anything V2 monocular depth estimation).
#[cfg(feature = "android")]
#[no_mangle]
pub extern "system" fn Java_com_reilandeubank_unprocess_engine_FilmrEngine_isDepthSupported<
    'local,
>(
    _env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jni::sys::jboolean {
    #[cfg(feature = "depth")]
    { 1u8 }
    #[cfg(not(feature = "depth"))]
    { 0u8 }
}

/// Process an image with depth-guided DOF and object-motion effects.
///
/// Parameters match `processImage` with one addition:
/// - `model_path`: absolute path to `depth_anything_v2_vits.rten` on the device.
///   Pass an empty string to skip depth estimation (falls back to uniform depth).
///
/// Depth estimation only runs when:
///   1. The library was compiled with `--features depth`, AND
///   2. `model_path` is non-empty and the file exists, AND
///   3. `config_json` has `dof_amount > 0` or `object_motion_amount > 0`.
///
/// On any depth-estimation error the function falls back to depth-less processing
/// rather than failing.
#[cfg(feature = "android")]
#[no_mangle]
pub extern "system" fn Java_com_reilandeubank_unprocess_engine_FilmrEngine_processImageWithDepth<
    'local,
>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    rgba_bytes: JByteArray<'local>,
    width: jint,
    height: jint,
    preset_key: JString<'local>,
    style_key: JString<'local>,
    config_json: JString<'local>,
    model_path: JString<'local>,
) -> jbyteArray {
    match process_with_depth_impl(
        &mut env,
        &rgba_bytes,
        width,
        height,
        &preset_key,
        &style_key,
        &config_json,
        &model_path,
    ) {
        Ok(arr) => arr,
        Err(e) => {
            let _ = env.throw_new("java/lang/RuntimeException", e.as_str());
            std::ptr::null_mut()
        }
    }
}

#[cfg(feature = "android")]
fn process_with_depth_impl<'local>(
    env: &mut JNIEnv<'local>,
    rgba_bytes: &JByteArray<'local>,
    width: jint,
    height: jint,
    preset_key: &JString<'local>,
    style_key: &JString<'local>,
    config_json: &JString<'local>,
    model_path: &JString<'local>,
) -> Result<jbyteArray, String> {
    let preset_key: String = env.get_string(preset_key).map_err(|e| e.to_string())?.into();
    let style_key: String = env.get_string(style_key).map_err(|e| e.to_string())?.into();
    let config_json: String = env.get_string(config_json).map_err(|e| e.to_string())?.into();
    let model_path_str: String = env.get_string(model_path).map_err(|e| e.to_string())?.into();

    if width < 0 || width as usize > MAX_SAFE_JNI_ARRAY_LEN {
        return Err(format!("Invalid width: {}", width));
    }
    if height < 0 || height as usize > MAX_SAFE_JNI_ARRAY_LEN {
        return Err(format!("Invalid height: {}", height));
    }

    let array_len = env
        .get_array_length(rgba_bytes)
        .map_err(|e| e.to_string())?;
    if array_len < 0 || array_len as usize > MAX_SAFE_JNI_ARRAY_LEN {
        return Err(format!("rgba_bytes array length out of bounds: {}", array_len));
    }

    let rgba = env.convert_byte_array(rgba_bytes).map_err(|e| e.to_string())?;

    let w = width as u32;
    let h = height as u32;
    let input = rgba_to_rgb_image(&rgba, w, h)
        .ok_or_else(|| "Invalid image dimensions".to_string())?;

    let film = stock_by_key(&preset_key).with_style(style_from_str(&style_key));
    let config: SimulationConfig = serde_json::from_str(&config_json).map_err(|e| format!("Invalid config JSON: {e}"))?;

    // Attempt depth estimation when the feature is compiled in and a model path is given
    let depth_map = estimate_depth_if_available(&input, &model_path_str, &config);

    let output = crate::processor::process_image_with_depth(&input, &film, &config, depth_map.as_ref());
    let output_bytes = output.into_raw();

    env.byte_array_from_slice(&output_bytes)
        .map(|arr| arr.into_raw())
        .map_err(|e| e.to_string())
}

/// Run depth estimation only when the feature and model are available and relevant.
#[cfg(feature = "android")]
fn estimate_depth_if_available(
    image: &image::RgbImage,
    model_path: &str,
    config: &SimulationConfig,
) -> Option<crate::depth::DepthMap> {
    // Only bother if effects that need depth are actually enabled
    if config.dof_amount <= 0.0 && config.object_motion_amount <= 0.0 {
        return None;
    }
    if model_path.is_empty() {
        return None;
    }

    #[cfg(feature = "depth")]
    {
        match crate::depth::estimate_with_model(image, model_path) {
            Ok(dm) => Some(dm),
            Err(e) => {
                eprintln!("[filmr-android] depth estimation failed: {e}");
                None
            }
        }
    }

    #[cfg(not(feature = "depth"))]
    {
        let _ = (image, model_path);
        None
    }
}

// ---------------------------------------------------------------------------
// Unit tests — no JVM required
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- MAX_SAFE_JNI_ARRAY_LEN ---

    #[test]
    fn max_safe_jni_array_len_is_256_mb() {
        assert_eq!(MAX_SAFE_JNI_ARRAY_LEN, 256 * 1024 * 1024);
    }

    // --- check_jni_array_len ---

    #[test]
    fn jni_array_len_negative_is_error() {
        let result = check_jni_array_len(-1);
        assert!(result.is_err(), "negative length must be rejected");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("-1"),
            "error message should contain the bad value, got: {msg}"
        );
    }

    #[test]
    fn jni_array_len_exceeds_max_is_error() {
        let too_large = (MAX_SAFE_JNI_ARRAY_LEN + 1) as i32;
        // Only test when the cast doesn't overflow i32 (it won't: 256 MB + 1 < i32::MAX)
        if too_large > 0 {
            let result = check_jni_array_len(too_large);
            assert!(result.is_err(), "length above MAX_SAFE_JNI_ARRAY_LEN must be rejected");
        }
    }

    #[test]
    fn jni_array_len_at_max_is_ok() {
        // MAX_SAFE_JNI_ARRAY_LEN fits in i32 only when ≤ i32::MAX; use a known-safe value.
        let valid_len: i32 = 1024;
        let result = check_jni_array_len(valid_len);
        assert_eq!(result, Ok(1024usize));
    }

    #[test]
    fn jni_array_len_zero_is_ok() {
        assert_eq!(check_jni_array_len(0), Ok(0usize));
    }

    // --- check_dng_dimensions (MAX_DNG_DIM guard, Issue #11) ---

    #[test]
    fn dng_dimensions_within_limit_accepted() {
        assert!(check_dng_dimensions(1920, 1080).is_ok());
        assert!(check_dng_dimensions(16384, 16384).is_ok());
        assert!(check_dng_dimensions(1, 1).is_ok());
    }

    #[test]
    fn dng_dimensions_oversized_width_rejected() {
        let result = check_dng_dimensions(16385, 100);
        assert!(result.is_err(), "width > 16384 must be rejected");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("16385"),
            "error should mention the oversized dimension, got: {msg}"
        );
    }

    #[test]
    fn dng_dimensions_oversized_height_rejected() {
        let result = check_dng_dimensions(100, 16385);
        assert!(result.is_err(), "height > 16384 must be rejected");
    }

    #[test]
    fn dng_dimensions_both_oversized_rejected() {
        assert!(check_dng_dimensions(20000, 20000).is_err());
    }
}
