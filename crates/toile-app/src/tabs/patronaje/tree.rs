use eframe::egui::{self, Align2, FontId, Rect, Response, Sense, Vec2, pos2, vec2};
use toile_engine::draft::{Draft, PieceKey};

use super::state::Scope;
use crate::glyph;
use crate::theme::Theme;
use crate::widgets::{PAD, section, tree_row};

const PRODUCT_ICON: &str = "2 3 9 3 9 11 2 11 2 3; 7 6 14 6 14 14 7 14 7 6";
const PIECE_ICON: &str = "4 2 10 2 13 6 13 14 4 14 4 2";
const PLUS: &str = "8 3 8 13; 3 8 13 8";
const CROSS: &str = "5 5 11 11; 5 11 11 5";
const PENCIL: &str = "3 13 4 9 10 3 13 6 7 12 3 13; 9 5 11 7";
const ROW_H: f32 = 26.0;

/// What the row that stands for the whole product is called.
const WHOLE: &str = "Todas las piezas";

/// How far a piece row sits in from the product it belongs to.
const INDENT: f32 = PAD;

/// What the tree asks of the tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plea {
    /// Start drawing a new piece on the mat.
    Draw,
    /// Show the whole product: every piece at once.
    Overview,
    /// Open one piece on its own: the one the mat draws and the panels edit.
    Focus(PieceKey),
    /// Give a piece a new name, once one has been typed.
    Rename(PieceKey, String),
    /// Take a piece off the table.
    ///
    /// Right away, with no modal: the status bar names the undo that brings
    /// the piece back, and that is the whole of the confirmation.
    Remove(PieceKey),
}

/// The product tree: the whole product, the pieces it holds, and the way to a
/// new one.
///
/// The first row is the product itself, lit while the mat shows every piece;
/// a press on it goes back there from any piece. Under it the piece in front
/// is lit, a click on a piece row opens that piece on its own, the pencil
/// renames it in place and the cross takes it off. The "+ Pieza" row answers
/// with a document or without one: over an empty table the tab turns the plea
/// into the question that has to come first, which product is this. A row that
/// leads somewhere beats a row that only looks as though it would.
///
/// `drawing` is whether a piece is being drawn already, which is what keeps the
/// row lit for as long as the drawing it opened is on the mat.
pub fn product(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: Option<&Draft>,
    active: Option<PieceKey>,
    renaming: &mut Option<(PieceKey, String)>,
    drawing: bool,
    scope: Scope,
) -> Option<Plea> {
    section(ui, theme, "Producto");
    let mut asked = None;
    if draft.is_some() {
        let whole = tree_row(ui, theme, WHOLE, scope == Scope::Product, 0.0, |p, r, c| {
            glyph::paint(p, r, c, PRODUCT_ICON);
        });
        if whole.clicked() {
            asked = Some(Plea::Overview);
        }
    }
    let pieces = draft.map(|draft| draft.doc().pieces.iter().collect::<Vec<_>>());
    for &(key, piece) in &pieces.unwrap_or_default() {
        if matches!(renaming, Some((editing, _)) if *editing == key) {
            let buffer = &mut renaming.as_mut().expect("a name is being typed").1;
            if let Some(commit) = editing_row(ui, theme, buffer) {
                let name = buffer.trim().to_owned();
                let renamed = commit && !name.is_empty() && name != piece.name;
                *renaming = None;
                if renamed {
                    asked = Some(Plea::Rename(key, name));
                }
            }
            continue;
        }
        let lit = active == Some(key);
        let row = tree_row(ui, theme, &piece.name, lit, INDENT, |p, r, c| {
            glyph::paint(p, r, c, PIECE_ICON);
        });
        // Both icons paint on hover; a rename in flight wins any stray click a
        // blur may raise on another row, so it is never lost.
        let remove = removal(ui, theme, &row, key);
        let rename = rename_pencil(ui, theme, &row, key);
        if asked.is_some() {
            continue;
        }
        if remove {
            asked = Some(Plea::Remove(key));
        } else if rename {
            *renaming = Some((key, piece.name.clone()));
        } else if row.clicked() {
            asked = Some(Plea::Focus(key));
        }
    }
    // The rename wins here too: the click that leaves the field is the click
    // that lands on this row, and a name already typed is worth more than a
    // press the person can simply repeat.
    if plus_row(ui, theme, "Pieza", drawing).clicked() && asked.is_none() {
        asked = Some(Plea::Draw);
    }
    asked
}

/// A piece row turned into a field: the name is edited where it is read,
/// committed on Enter or when the field is left, abandoned on Escape.
///
/// `Some(true)` commits, `Some(false)` abandons, `None` keeps the field open.
fn editing_row(ui: &mut egui::Ui, theme: &Theme, buffer: &mut String) -> Option<bool> {
    let row = tree_row(ui, theme, "", true, INDENT, |p, r, c| {
        glyph::paint(p, r, c, PIECE_ICON);
    });
    let field = Rect::from_min_max(
        pos2(row.rect.left() + 40.0 + INDENT, row.rect.top() + 3.0),
        pos2(row.rect.right() - PAD, row.rect.bottom() - 3.0),
    );
    let edit = ui.put(field, egui::TextEdit::singleline(buffer));
    // A field just opened has not been focused yet; it is asked for here, and
    // the moment it is lost — to Enter, Escape or a click away — the edit ends.
    if edit.lost_focus() {
        return Some(!ui.input(|i| i.key_pressed(egui::Key::Escape)));
    }
    edit.request_focus();
    None
}

/// The pencil at the right of a hovered row, and whether it was pressed: it
/// opens the row for renaming in place.
fn rename_pencil(ui: &mut egui::Ui, theme: &Theme, row: &Response, key: PieceKey) -> bool {
    let spot = Rect::from_center_size(
        row.rect.right_center() - vec2(PAD + 28.0, 0.0),
        Vec2::splat(16.0),
    );
    let hit = ui.interact(
        spot,
        ui.id()
            .with(("renombrar-pieza", key.index(), key.generation())),
        Sense::click(),
    );
    if row.hovered() || hit.hovered() {
        let ink = if hit.hovered() {
            theme.accent
        } else {
            theme.muted
        };
        glyph::paint(ui.painter(), spot, ink, PENCIL);
    }
    hit.clicked()
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

/// The "add a piece" row: it asks for the drawing gesture on the mat.
///
/// It rests in `ink_soft`, the ink of a control waiting to be used, and never
/// in `muted`, which is what a tile whose phase has not arrived is drawn in.
///
/// It lights the way a chosen piece row does, and it lights on the press
/// rather than after it: the drawing it opens has nothing to show until the
/// first vertex lands and the status bar is drawn before the tabs, so for one
/// frame this row is the only place the press can be seen at all.
fn plus_row(ui: &mut egui::Ui, theme: &Theme, label: &str, armed: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::click());
    let lit = armed || resp.clicked() || resp.is_pointer_button_down_on();
    let tint = if lit {
        0.16
    } else if resp.hovered() {
        0.07
    } else {
        0.0
    };
    let ink = if lit || resp.hovered() {
        theme.ink
    } else {
        theme.ink_soft
    };
    let p = ui.painter();
    if tint > 0.0 {
        p.rect_filled(rect, 0.0, theme.accent.gamma_multiply(tint));
    }
    let slot = Rect::from_center_size(rect.left_center() + vec2(34.0, 0.0), Vec2::splat(16.0));
    glyph::paint(p, slot, ink, PLUS);
    let at = rect.left_center() + vec2(50.0, 0.0);
    let font = FontId::proportional(13.0);
    p.text(at, Align2::LEFT_CENTER, label, font, ink);
    resp
}
