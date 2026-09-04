#![allow(
    dead_code,
    reason = "the panels that consume these helpers land tab by tab"
)]

mod canvas;
mod control;
mod field;
mod panel;

pub use canvas::{canvas_label, fill, grid, mat_canvas};
pub use control::{button_icon, button_primary, button_secondary, select};
use eframe::egui::CornerRadius;
pub use field::{field_row, formula_row, formula_row_fault};
pub use panel::{footer_note, list_row_icon, rule, section, section_with, tree_row};

/// Horizontal breathing room inside a side panel, in points.
pub(crate) const PAD: f32 = 12.0;
pub(crate) const CORNER: CornerRadius = CornerRadius::same(2);
