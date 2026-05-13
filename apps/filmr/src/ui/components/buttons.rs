//! Reusable button constructors.

use super::tokens::*;
use egui::{self, RichText, Stroke};

/// Small action button (11px, 4r, 24h) — for Add/Clear/Import/Export etc.
pub fn action_button(label: &str) -> egui::Button<'_> {
    egui::Button::new(
        RichText::new(label)
            .size(SMALL_FONT_SIZE)
            .color(TEXT_SECONDARY),
    )
    .fill(BG_MEDIUM)
    .stroke(Stroke::NONE)
    .corner_radius(4.0)
    .min_size(egui::vec2(0.0, ACTION_BTN_HEIGHT))
}

/// Toolbar button (13px, 6r, 28h) — for Compare/Split/Save/Metrics/⚙.
pub fn toolbar_button(label: &str, selected: bool) -> egui::Button<'_> {
    egui::Button::new(
        RichText::new(label)
            .size(TOOLBAR_FONT_SIZE)
            .color(TEXT_SECONDARY),
    )
    .fill(BG_MEDIUM)
    .stroke(Stroke::NONE)
    .corner_radius(6.0)
    .min_size(egui::vec2(0.0, TOOLBAR_BTN_HEIGHT))
    .selected(selected)
}

/// Primary action button (accent fill, dark text, bold).
pub fn primary_button(label: &str) -> egui::Button<'_> {
    egui::Button::new(
        RichText::new(label)
            .size(TOOLBAR_FONT_SIZE)
            .strong()
            .color(TEXT_DARK),
    )
    .fill(ACCENT)
    .stroke(Stroke::NONE)
    .corner_radius(6.0)
    .min_size(egui::vec2(0.0, TOOLBAR_BTN_HEIGHT))
}

/// Toolbar vertical separator.
pub fn toolbar_separator(ui: &mut egui::Ui) {
    ui.add_space(6.0);
    let (r, _) = ui.allocate_exact_size(egui::vec2(2.0, 18.0), egui::Sense::hover());
    ui.painter().rect_filled(r, 1.0, BORDER);
    ui.add_space(6.0);
}
