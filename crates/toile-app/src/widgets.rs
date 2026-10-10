mod canvas;
mod cite;
mod control;
mod field;
mod panel;
mod readout;
mod slider;

pub use canvas::{canvas_label, fill, grid, mat_canvas};
pub use cite::{Mention, Named, measure_row, named_formula_row};
pub use control::{
    button_ghost_icon, button_icon, button_named, button_primary, button_secondary, check_named,
    ghost_icon_room,
};
use eframe::egui::CornerRadius;
pub use field::{Editable, Edited, field_row, formula_row, typed_row};
pub use panel::{
    alert_note, footer_note, list_row_icon, list_row_named, plain_note, section, section_with,
    tree_row,
};
pub use readout::{cycle, cycle_named, readout, readout_room};
pub use slider::{Track, track};

/// Horizontal breathing room inside a side panel, in points.
pub(crate) const PAD: f32 = 12.0;
pub(crate) const CORNER: CornerRadius = CornerRadius::same(2);
