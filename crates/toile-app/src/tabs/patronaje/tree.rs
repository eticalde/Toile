use eframe::egui::{self, Align2, FontId, Rect, Sense, Vec2, vec2};
use toile_engine::draft::{Draft, PieceKey};

use crate::glyph;
use crate::theme::Theme;
use crate::widgets::{section, tree_row};

const PIECE_ICON: &str = "4 2 10 2 13 6 13 14 4 14 4 2";
const PLUS: &str = "8 3 8 13; 3 8 13 8";

/// The product tree: the pieces the document draws.
///
/// An empty document shows an empty tree on purpose; the ways onto the table
/// are on the mat, where a person looking at nothing is already looking.
pub fn product(ui: &mut egui::Ui, theme: &Theme, draft: Option<&Draft>, drawn: Option<PieceKey>) {
    section(ui, theme, "Producto");
    let pieces = draft.map(|draft| draft.doc().pieces.iter().collect::<Vec<_>>());
    for &(key, piece) in &pieces.unwrap_or_default() {
        tree_row(
            ui,
            theme,
            &piece.name,
            drawn == Some(key),
            0.0,
            |p, r, c| {
                glyph::paint(p, r, c, PIECE_ICON);
            },
        );
    }
    ghost_row(ui, theme, "Pieza");
}

/// The "add a piece" affordance: a hint rather than an item, and inert until
/// the tool that draws one exists, so it does not answer a click with nothing.
fn ghost_row(ui: &mut egui::Ui, theme: &Theme, label: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::hover());
    let p = ui.painter();
    let slot = Rect::from_center_size(rect.left_center() + vec2(34.0, 0.0), Vec2::splat(16.0));
    glyph::paint(p, slot, theme.muted, PLUS);
    let at = rect.left_center() + vec2(50.0, 0.0);
    let font = FontId::proportional(13.0);
    p.text(at, Align2::LEFT_CENTER, label, font, theme.muted);
}
