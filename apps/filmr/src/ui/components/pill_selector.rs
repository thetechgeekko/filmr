//! Pill selector — segmented control component.

use super::tokens::*;
use egui::{self, Align2, FontId, Sense};

/// Renders a single-row pill selector. Returns `true` if the value changed.
pub fn pill_selector<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    id_salt: &str,
    current: &mut T,
    options: &[(T, &str)],
) -> bool {
    let pill_r = PILL_HEIGHT / 2.0;
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, PILL_HEIGHT), Sense::hover());
    ui.painter().rect_filled(rect, pill_r, BG_TRACK);

    let n = options.len() as f32;
    let seg_w = w / n;
    let mut changed = false;

    // Selected highlight
    if let Some(sel_i) = options.iter().position(|(v, _)| *v == *current) {
        let sr = egui::Rect::from_min_size(
            egui::pos2(rect.left() + sel_i as f32 * seg_w, rect.top()),
            egui::vec2(seg_w, PILL_HEIGHT),
        );
        ui.painter().rect_filled(sr, pill_r, ACCENT);
    }

    // Segments
    for (i, (value, label)) in options.iter().enumerate() {
        let is_sel = *current == *value;
        let sr = egui::Rect::from_min_size(
            egui::pos2(rect.left() + i as f32 * seg_w, rect.top()),
            egui::vec2(seg_w, PILL_HEIGHT),
        );
        let resp = ui.interact(sr, ui.id().with((id_salt, i)), Sense::click());

        // Hover feedback on non-selected
        if !is_sel && resp.hovered() {
            ui.painter().rect_filled(sr, pill_r, BG_HOVER);
        }

        // Label
        ui.painter().text(
            sr.center(),
            Align2::CENTER_CENTER,
            *label,
            FontId::proportional(SMALL_FONT_SIZE),
            if is_sel { TEXT_DARK } else { TEXT_SECONDARY },
        );

        if resp.clicked() {
            *current = *value;
            changed = true;
        }
    }

    changed
}

/// Renders a multi-row pill selector (for when options don't fit one row).
/// `rows` is a slice of slices of (value, label) pairs.
pub fn pill_selector_rows<T: PartialEq + Copy>(
    ui: &mut egui::Ui,
    id_salt: &str,
    current: &mut T,
    rows: &[&[(T, &str)]],
) -> bool {
    let mut changed = false;
    let mut global_idx = 0usize;
    let pill_r = PILL_HEIGHT / 2.0;

    for row in rows {
        let w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(egui::vec2(w, PILL_HEIGHT), Sense::hover());
        ui.painter().rect_filled(rect, pill_r, BG_TRACK);

        let n = row.len() as f32;
        let seg_w = w / n;

        // Selected highlight
        if let Some(sel_i) = row.iter().position(|(v, _)| *v == *current) {
            let sr = egui::Rect::from_min_size(
                egui::pos2(rect.left() + sel_i as f32 * seg_w, rect.top()),
                egui::vec2(seg_w, PILL_HEIGHT),
            );
            ui.painter().rect_filled(sr, pill_r, ACCENT);
        }

        for (i, (value, label)) in row.iter().enumerate() {
            let is_sel = *current == *value;
            let sr = egui::Rect::from_min_size(
                egui::pos2(rect.left() + i as f32 * seg_w, rect.top()),
                egui::vec2(seg_w, PILL_HEIGHT),
            );
            let resp = ui.interact(sr, ui.id().with((id_salt, global_idx)), Sense::click());

            if !is_sel && resp.hovered() {
                ui.painter().rect_filled(sr, pill_r, BG_HOVER);
            }

            ui.painter().text(
                sr.center(),
                Align2::CENTER_CENTER,
                *label,
                FontId::proportional(SMALL_FONT_SIZE),
                if is_sel { TEXT_DARK } else { TEXT_SECONDARY },
            );

            if resp.clicked() {
                *current = *value;
                changed = true;
            }
            global_idx += 1;
        }
        ui.add_space(3.0);
    }

    changed
}
