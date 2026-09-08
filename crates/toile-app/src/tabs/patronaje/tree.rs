use eframe::egui::{self, Align2, FontId, Rect, Response, Sense, Vec2, pos2, vec2};
use toile_engine::draft::{Draft, PieceKey};

use crate::glyph;
use crate::theme::Theme;
use crate::widgets::{PAD, section, tree_row};

const PIECE_ICON: &str = "4 2 10 2 13 6 13 14 4 14 4 2";
const PLUS: &str = "8 3 8 13; 3 8 13 8";
const CROSS: &str = "5 5 11 11; 5 11 11 5";
const PENCIL: &str = "3 13 4 9 10 3 13 6 7 12 3 13; 9 5 11 7";
const ROW_H: f32 = 26.0;

/// What the tree asks of the tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plea {
    /// Start drawing a new piece on the mat.
    Draw,
    /// Bring a piece to the front: the one the mat draws and the panels edit.
    Focus(PieceKey),
    /// Give a piece a new name, once one has been typed.
    Rename(PieceKey, String),
    /// Take a piece off the table.
    ///
    /// Right away, with no modal: the status bar names the undo that brings
    /// the piece back, and that is the whole of the confirmation.
    Remove(PieceKey),
}

/// The product tree: the pieces the document draws, and the way to a new one.
///
/// The active piece — the one the mat is drawing — is lit; a click on any other
/// row brings it to the front, the pencil renames it in place, the cross takes
/// it off. With no document there is nothing to add a piece to, so the "+
/// Pieza" row stays a hint: the ways onto the table are on the mat, where a
/// person looking at nothing is already looking.
pub fn product(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: Option<&Draft>,
    active: Option<PieceKey>,
    renaming: &mut Option<(PieceKey, String)>,
) -> Option<Plea> {
    section(ui, theme, "Producto");
    let mut asked = None;
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
        let row = tree_row(
            ui,
            theme,
            &piece.name,
            active == Some(key),
            0.0,
            |p, r, c| {
                glyph::paint(p, r, c, PIECE_ICON);
            },
        );
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
    if draft.is_some() {
        if plus_row(ui, theme, "Pieza").clicked() {
            asked = Some(Plea::Draw);
        }
    } else {
        ghost_row(ui, theme, "Pieza");
    }
    asked
}

/// A piece row turned into a field: the name is edited where it is read,
/// committed on Enter or when the field is left, abandoned on Escape.
///
/// `Some(true)` commits, `Some(false)` abandons, `None` keeps the field open.
fn editing_row(ui: &mut egui::Ui, theme: &Theme, buffer: &mut String) -> Option<bool> {
    let row = tree_row(ui, theme, "", true, 0.0, |p, r, c| {
        glyph::paint(p, r, c, PIECE_ICON);
    });
    let field = Rect::from_min_max(
        pos2(row.rect.left() + 40.0, row.rect.top() + 3.0),
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
