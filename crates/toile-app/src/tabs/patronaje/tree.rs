use eframe::egui::{self, Align2, FontId, Rect, Response, Sense, Vec2, vec2};
use toile_engine::draft::{Draft, PieceKey};

use crate::glyph;
use crate::theme::Theme;
use crate::widgets::{PAD, section, tree_row};

const PIECE_ICON: &str = "4 2 10 2 13 6 13 14 4 14 4 2";
const PLUS: &str = "8 3 8 13; 3 8 13 8";
const CROSS: &str = "5 5 11 11; 5 11 11 5";
const ROW_H: f32 = 26.0;

/// What the tree asks of the tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plea {
    /// Start drawing a new piece on the mat.
    Draw,
    /// Take a piece off the table.
    ///
    /// Right away, with no modal: the status bar names the undo that brings
    /// the piece back, and that is the whole of the confirmation.
    Remove(PieceKey),
}

/// The product tree: the pieces the document draws, and the way to a new one.
///
/// With no document there is nothing to add a piece to, so the "+ Pieza" row
/// stays a hint: the ways onto the table are on the mat, where a person
/// looking at nothing is already looking.
pub fn product(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: Option<&Draft>,
    drawn: Option<PieceKey>,
) -> Option<Plea> {
    section(ui, theme, "Producto");
    let mut asked = None;
    let pieces = draft.map(|draft| draft.doc().pieces.iter().collect::<Vec<_>>());
    for &(key, piece) in &pieces.unwrap_or_default() {
        let row = tree_row(
            ui,
            theme,
            &piece.name,
            drawn == Some(key),
            0.0,
            |p, r, c| {
                glyph::paint(p, r, c, PIECE_ICON);
            },
        );
        if removal(ui, theme, &row, key) {
            asked = Some(Plea::Remove(key));
        }
    }
    if draft.is_some() {
        if plus_row(ui, theme, "Pieza").clicked() {
            asked = Some(Plea::Draw);
        }
    } else {
        ghost_row(ui, theme, "Pieza");
    }
    asked
}

/// The cross at the right of a hovered row, and whether it was pressed.
///
/// It lives on the row rather than in a menu so that taking a piece off the
/// table costs one aimed click — the undo named in the status bar is what
/// stands in for a confirmation.
fn removal(ui: &mut egui::Ui, theme: &Theme, row: &Response, key: PieceKey) -> bool {
    let spot = Rect::from_center_size(
        row.rect.right_center() - vec2(PAD + 8.0, 0.0),
        Vec2::splat(16.0),
    );
    let hit = ui.interact(
        spot,
        ui.id()
            .with(("quitar-pieza", key.index(), key.generation())),
        Sense::click(),
    );
    if row.hovered() || hit.hovered() {
        let ink = if hit.hovered() {
            theme.alert
        } else {
            theme.muted
        };
        glyph::paint(ui.painter(), spot, ink, CROSS);
    }
    hit.clicked()
}

/// The live "add a piece" row: it starts the drawing gesture on the mat.
fn plus_row(ui: &mut egui::Ui, theme: &Theme, label: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::click());
    let ink = if resp.hovered() {
        theme.ink_soft
    } else {
        theme.muted
    };
    if resp.hovered() {
        ui.painter()
            .rect_filled(rect, 0.0, theme.accent.gamma_multiply(0.07));
    }
    paint_row(ui, rect, label, ink);
    resp
}

/// The same row, inert: a hint of what a document would allow.
fn ghost_row(ui: &mut egui::Ui, theme: &Theme, label: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::hover());
    paint_row(ui, rect, label, theme.muted);
}

fn paint_row(ui: &egui::Ui, rect: Rect, label: &str, ink: egui::Color32) {
    let p = ui.painter();
    let slot = Rect::from_center_size(rect.left_center() + vec2(34.0, 0.0), Vec2::splat(16.0));
    glyph::paint(p, slot, ink, PLUS);
    let at = rect.left_center() + vec2(50.0, 0.0);
    let font = FontId::proportional(13.0);
    p.text(at, Align2::LEFT_CENTER, label, font, ink);
}
