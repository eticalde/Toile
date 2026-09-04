use eframe::egui::{self, Align2, FontId, Rect, Sense, Vec2, vec2};
use toile_engine::draft::{Draft, PieceKey, block};

use super::state::State;
use crate::glyph;
use crate::theme::Theme;
use crate::widgets::{rule, section, tree_row};

const PIECE_ICON: &str = "4 2 10 2 13 6 13 14 4 14 4 2";
const SHEET_ICON: &str = "3 2 13 2 13 14 3 14 3 2; 6 6 10 6; 6 9 10 9";
const PLUS: &str = "8 3 8 13; 3 8 13 8";

/// The way in for the block Toile brings, until opening a file replaces it.
const EXAMPLE: &str = "Ejemplo · pantalón base";

/// The product tree: the pieces the document draws, and the ways to gain one.
///
/// An empty document shows an empty table on purpose: the application does not
/// decide what a person is drafting.
pub fn product(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: Option<&Draft>,
    drawn: Option<PieceKey>,
    state: &mut State,
) {
    section(ui, theme, "Producto");
    let pieces = draft.map(|draft| draft.doc().pieces.iter().collect::<Vec<_>>());
    let pieces = pieces.unwrap_or_default();
    for &(key, piece) in &pieces {
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
    if pieces.is_empty() {
        rule(ui, theme);
        let row = tree_row(ui, theme, EXAMPLE, false, 0.0, |p, r, c| {
            glyph::paint(p, r, c, SHEET_ICON);
        });
        if row.clicked() {
            state.pending = Some(block::trouser_front());
        }
    }
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
