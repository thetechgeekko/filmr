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
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jstring {
    let config = SimulationConfig::default();
    let json = serde_json::to_string(&config).unwrap_or_else(|_| "{}".to_string());
    env.new_string(json)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
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
