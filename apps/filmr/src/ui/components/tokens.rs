//! Design tokens — single source of truth for all UI colors and sizes.

use egui::Color32;

// ── Colors ──
pub const ACCENT: Color32 = Color32::from_rgb(230, 155, 50);
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(245, 175, 70);
pub const BG_DARKEST: Color32 = Color32::from_rgb(24, 24, 28);
pub const BG_DARK: Color32 = Color32::from_rgb(32, 32, 36);
pub const BG_MEDIUM: Color32 = Color32::from_rgb(42, 42, 48);
pub const BG_HOVER: Color32 = Color32::from_rgb(52, 52, 60);
pub const BG_TRACK: Color32 = Color32::from_rgb(36, 36, 40);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(220, 220, 225);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(150, 150, 160);
pub const TEXT_DISABLED: Color32 = Color32::from_rgb(90, 90, 100);
pub const TEXT_DARK: Color32 = Color32::from_rgb(24, 24, 28);
pub const BORDER: Color32 = Color32::from_rgb(55, 55, 65);

// ── Sizes ──
pub const PILL_HEIGHT: f32 = 26.0;
pub const TOOLBAR_BTN_HEIGHT: f32 = 28.0;
pub const ACTION_BTN_HEIGHT: f32 = 24.0;
pub const TOOLBAR_FONT_SIZE: f32 = 13.0;
pub const BODY_FONT_SIZE: f32 = 12.0;
pub const SMALL_FONT_SIZE: f32 = 11.0;
