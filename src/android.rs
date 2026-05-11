//! Android JNI bindings for the filmr engine.
//!
//! Exposes the filmr film-simulation pipeline to Android via JNI.
//! Enable with `--features android` when cross-compiling for Android.
//!
//! The Kotlin counterpart lives in:
//!   com.reilandeubank.unprocess.engine.FilmrEngine

use crate::film::{FilmStock, FilmStyle};
use crate::processor::{process_image, SimulationConfig};
use image::{ImageBuffer, RgbImage};
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jbyteArray, jint, jstring};
use jni::JNIEnv;

// ---------------------------------------------------------------------------
// Preset lookup
// ---------------------------------------------------------------------------

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

    let rgba = env
        .convert_byte_array(rgba_bytes)
        .map_err(|e| e.to_string())?;

    let w = width as u32;
    let h = height as u32;
    let input =
        rgba_to_rgb_image(&rgba, w, h).ok_or_else(|| "Invalid image dimensions".to_string())?;

    let film = stock_by_key(&preset_key).with_style(style_from_str(&style_key));

    let config: SimulationConfig =
        serde_json::from_str(&config_json).unwrap_or_default();

    let output = process_image(&input, &film, &config);
    let output_bytes = output.into_raw();

    env.byte_array_from_slice(&output_bytes)
        .map(|arr| arr.into_raw())
        .map_err(|e| e.to_string())
}

/// Return a JSON array of all available presets.
///
/// Each element: `{"key":"KODAK_PORTRA_400","manufacturer":"Kodak","name":"Portra 400","iso":400}`
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
///   - Tag 33422 / 0x828D — CFAPattern (2×2 Bayer mosaic, default RGGB)
///   - Tag 50714 / 0xC5BA — BlackLevel (per-channel; scalar fallback = 0)
///   - Tag 50717 / 0xC5BD — WhiteLevel (scalar; default 65535 for 16-bit)
///   - BitsPerSample         — used to scale U8 data to 16-bit range
///
/// The demosaic is a simple bilinear interpolation (Malvar-class quality is
/// not required here because filmr will substantially remap the tones anyway).
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
    const TAG_CFA_PATTERN: u16 = 0x828D; // 33422
    // DNG spec tag numbers
    const TAG_BLACK_LEVEL_CORRECT: u16 = 0xC61A; // 50714

    const TAG_WHITE_LEVEL: u16 = 0xC61D; // 50717

    let cursor = Cursor::new(dng);
    let mut decoder = Decoder::new(cursor).map_err(|e| format!("TIFF decode error: {e}"))?;

    let (width, height) = decoder.dimensions().map_err(|e| format!("TIFF dimensions error: {e}"))?;

    // --- BitsPerSample ---
    let bits_per_sample: u32 = decoder
        .find_tag(Tag::BitsPerSample)
        .ok()
        .flatten()
        .and_then(|v| v.into_u32().ok())
        .unwrap_or(16);

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
            // may be rational, u16, u32, or f32
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
        DecodingResult::U8(v) => v.iter().map(|&x| (x as f32 - black_level) / scale).collect(),
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
            "RAW data too short: got {} samples, expected {}×{}={}",
            samples.len(), w, h, w * h
        ));
    }

    // --- Bilinear Bayer demosaic ---
    // cfa[row%2 * 2 + col%2] → 0=R, 1=G, 2=B
    let mut rgb = vec![0f32; w * h * 3];

    // Helper: safe pixel fetch with border clamping
    let bayer = |row: isize, col: isize| -> f32 {
        let r = row.clamp(0, h as isize - 1) as usize;
        let c = col.clamp(0, w as isize - 1) as usize;
        samples[r * w + c]
    };

    for row in 0..h {
        for col in 0..w {
            let channel = cfa[(row % 2) * 2 + (col % 2)] as usize; // 0=R,1=G,2=B
            let r = row as isize;
            let c = col as isize;

            let (out_r, out_g, out_b) = match channel {
                0 => {
                    // At R pixel: R known, interpolate G and B
                    let r_val = bayer(r, c);
                    let g_val = (bayer(r-1,c) + bayer(r+1,c) + bayer(r,c-1) + bayer(r,c+1)) / 4.0;
                    let b_val = (bayer(r-1,c-1) + bayer(r-1,c+1) + bayer(r+1,c-1) + bayer(r+1,c+1)) / 4.0;
                    (r_val, g_val, b_val)
                }
                1 => {
                    // At G pixel: G known, interpolate R and B from neighbours
                    let g_val = bayer(r, c);
                    // Determine if G is on an R-row or B-row
                    let r_row = cfa[(row % 2) * 2] == 0; // first col in this row is R?
                    if r_row {
                        // G on R-row: R left/right, B above/below
                        let r_val = (bayer(r, c-1) + bayer(r, c+1)) / 2.0;
                        let b_val = (bayer(r-1, c) + bayer(r+1, c)) / 2.0;
                        (r_val, g_val, b_val)
                    } else {
                        // G on B-row: R above/below, B left/right
                        let r_val = (bayer(r-1, c) + bayer(r+1, c)) / 2.0;
                        let b_val = (bayer(r, c-1) + bayer(r, c+1)) / 2.0;
                        (r_val, g_val, b_val)
                    }
                }
                2 => {
                    // At B pixel: B known, interpolate G and R
                    let b_val = bayer(r, c);
                    let g_val = (bayer(r-1,c) + bayer(r+1,c) + bayer(r,c-1) + bayer(r,c+1)) / 4.0;
                    let r_val = (bayer(r-1,c-1) + bayer(r-1,c+1) + bayer(r+1,c-1) + bayer(r+1,c+1)) / 4.0;
                    (r_val, g_val, b_val)
                }
                _ => (bayer(r, c), bayer(r, c), bayer(r, c)),
            };

            let base = (row * w + col) * 3;
            rgb[base]     = out_r.clamp(0.0, 1.0);
            rgb[base + 1] = out_g.clamp(0.0, 1.0);
            rgb[base + 2] = out_b.clamp(0.0, 1.0);
        }
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
/// simulation is applied.
///
/// Parameters (from Kotlin):
/// - `dng_bytes`   : raw DNG file bytes
/// - `preset_key`  : preset identifier string (e.g. "KODAK_PORTRA_400")
/// - `style_key`   : style identifier string  (e.g. "ACCURATE", "ARTISTIC")
/// - `config_json` : JSON-encoded `SimulationConfig`
///
/// Returns a `ByteArray` where:
///   - Bytes 0–3: image width  as little-endian signed 32-bit integer
///   - Bytes 4–7: image height as little-endian signed 32-bit integer
///   - Bytes 8… : width×height×3 processed RGB bytes (R G B R G B …)
///
/// Throws `java.lang.RuntimeException` on any failure.
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
) -> jbyteArray {
    match process_raw_dng_impl(&mut env, &dng_bytes, &preset_key, &style_key, &config_json) {
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
) -> Result<jbyteArray, String> {
    let preset_key: String = env.get_string(preset_key).map_err(|e| e.to_string())?.into();
    let style_key: String = env.get_string(style_key).map_err(|e| e.to_string())?.into();
    let config_json: String = env.get_string(config_json).map_err(|e| e.to_string())?.into();

    let dng = env.convert_byte_array(dng_bytes).map_err(|e| e.to_string())?;

    // Decode DNG → demosaiced linear RGB (with dimension header)
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
    let config: SimulationConfig = serde_json::from_str(&config_json).unwrap_or_default();

    let output = process_image(&input, &film, &config);
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

    let rgba = env.convert_byte_array(rgba_bytes).map_err(|e| e.to_string())?;

    let w = width as u32;
    let h = height as u32;
    let input = rgba_to_rgb_image(&rgba, w, h)
        .ok_or_else(|| "Invalid image dimensions".to_string())?;

    let film = stock_by_key(&preset_key).with_style(style_from_str(&style_key));
    let config: SimulationConfig = serde_json::from_str(&config_json).unwrap_or_default();

    // Attempt depth estimation when the feature is compiled in and a model path is given
    let depth_map = estimate_depth_if_available(&input, &model_path_str, &config);

    let output = crate::processor::process_image_with_depth(&input, &film, &config, depth_map.as_ref());
    let output_bytes = output.into_raw();

    env.byte_array_from_slice(&output_bytes)
        .map(|arr| arr.into_raw())
        .map_err(|e| e.to_string())
}

/// Run depth estimation only when the feature and model are available and relevant.
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
