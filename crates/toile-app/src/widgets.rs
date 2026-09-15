mod canvas;
mod cite;
mod control;
mod field;
mod panel;

pub use canvas::{canvas_label, fill, grid, mat_canvas};
pub use cite::{Mention, Named, measure_row, named_formula_row};
pub use control::{
    button_ghost, button_ghost_icon, button_icon, button_primary, button_secondary, cycle, readout,
};
use eframe::egui::CornerRadius;
pub use field::{Editable, Edited, field_row, formula_row};
pub use panel::{footer_note, list_row_icon, list_row_noted, section, section_with, tree_row};

/// Horizontal breathing room inside a side panel, in points.
pub(crate) const PAD: f32 = 12.0;
pub(crate) const CORNER: CornerRadius = CornerRadius::same(2);
