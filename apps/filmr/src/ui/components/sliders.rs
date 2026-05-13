//! Custom slider with label+value row above, full-width slider below.

use super::tokens::*;
use egui::{self, Layout, RichText};

/// Labeled slider: label+value top row (justify-between), full-width slider below.
/// Returns `true` if the value changed.
pub fn labeled_slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    logarithmic: bool,
) -> bool {
    // Top row: label left, value right
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(label)
                .size(BODY_FONT_SIZE)
                .color(TEXT_SECONDARY),
        );
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(format!("{:.2}", *value))
                    .size(BODY_FONT_SIZE)
                    .color(TEXT_PRIMARY),
            );
        });
    });

    // Slider full width, no text
    let mut slider = egui::Slider::new(value, range).show_value(false);
    if logarithmic {
        slider = slider.logarithmic(true);
    }
    ui.spacing_mut().slider_width = ui.available_width();
    let changed = ui.add(slider).changed();
    ui.add_space(4.0);
    changed
}

/// Section header — uppercase, muted color.
pub fn section_header(ui: &mut egui::Ui, label: &str) {
    ui.label(
        RichText::new(label)
            .strong()
            .size(BODY_FONT_SIZE)
            .color(TEXT_DISABLED),
    );
    ui.add_space(8.0);
}

/// Spacious divider between sections.
pub fn section_divider(ui: &mut egui::Ui) {
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(8.0);
}
