use eframe::egui::{
    self, Align2, CursorIcon, FontId, Painter, Pos2, Rect, Sense, Stroke, pos2, vec2,
};
use toile_engine::draft::{Draft, PieceKey};

use super::gesture::Gesture;
use super::state::{Scope, State};
use crate::glyph;
use crate::theme::Theme;
use crate::widgets::{PAD, button_icon, button_secondary, canvas_label};

const TAG: &str = "2 4 10 4 14 8 10 12 2 12 2 4; o 5 8 1.3";

/// The first step of a piece's trail: the product the piece belongs to.
const PRODUCT: &str = "Producto";

/// The mono line over the mat, and the chips beside it.
///
/// The line reads as a trail. Over the whole product it only says where the
/// mat is; over one piece its first step is the way back to every piece, and a
/// press on it takes that way.
pub fn show(
    ui: &mut egui::Ui,
    theme: &Theme,
    rect: Rect,
    draft: Option<&Draft>,
    piece: Option<PieceKey>,
    state: &mut State,
) {
    let zoom = state.view.zoom_percent(ui.ctx().pixels_per_point());
    let tail = format!("{zoom:.0} % · cm");
    let label = rect.translate(vec2(22.0, 20.0));
    match (draft, state.scope) {
        (None, _) => {
            let line = format!("PATRÓN — mesa vacía · {tail}");
            canvas_label(ui.painter(), theme, label, &line);
        }
        (Some(_), Scope::Product) => {
            let line = format!("{PRODUCT} · todas las piezas · {tail}");
            canvas_label(ui.painter(), theme, label, &line);
        }
        (Some(draft), Scope::Piece) => {
            let rest = format!(" › {} · {tail}", name(draft, piece, state));
            trail(ui, theme, label.left_top() + vec2(10.0, 8.0), &rest, state);
        }
    }
    chips(ui, theme, rect, state);
}

/// The trail over one piece: the product, drawn as the way back while a press
/// on it can be one, and the rest of the line after it.
///
/// While a question waits on the mat, or the pointer is in the middle of a
/// gesture that holds the undo stack open, the step senses nothing and wears
/// the caption's own ink: leaving the piece then would leave the entry open.
fn trail(ui: &mut egui::Ui, theme: &Theme, at: Pos2, rest: &str, state: &mut State) {
    let font = FontId::monospace(11.0);
    let live = state.ask.is_none()
        && matches!(
            state.gesture,
            Gesture::Idle | Gesture::Drawing { .. } | Gesture::Tracing(_)
        );
    let size = ui
        .painter()
        .layout_no_wrap(PRODUCT.to_owned(), font.clone(), theme.muted)
        .size();
    let sense = if live { Sense::click() } else { Sense::hover() };
    let spot = Rect::from_min_size(at, size).expand(3.0);
    let hit = ui.interact(spot, ui.id().with("volver-al-producto"), sense);
    let hovered = live && hit.hovered();
    let ink = if hovered {
        theme.ink
    } else if live {
        theme.accent
    } else {
        theme.muted
    };
    let p = ui.painter();
    p.text(at, Align2::LEFT_TOP, PRODUCT, font.clone(), ink);
    if live {
        p.hline(
            at.x..=at.x + size.x,
            at.y + size.y + 1.0,
            Stroke::new(1.0, ink),
        );
    }
    p.text(
        at + vec2(size.x, 0.0),
        Align2::LEFT_TOP,
        rest,
        font,
        theme.muted,
    );
    if hovered {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if live && hit.clicked() {
        state.overview();
    }
}

/// What the piece on the mat is called; a contour not yet closed is a new
/// piece with no name yet.
fn name(draft: &Draft, piece: Option<PieceKey>, state: &State) -> String {
    if matches!(state.gesture, Gesture::Drawing { .. }) {
        return "pieza nueva".to_owned();
    }
    piece
        .and_then(|key| draft.doc().pieces.get(key))
        .map_or_else(|| "sin piezas".to_owned(), |held| held.name.clone())
}

/// The view chips, right to left along the top of the mat.
///
/// The label layer names nodes, and the whole product draws none, so there
/// the chip that switches it is not offered at all.
fn chips(ui: &mut egui::Ui, theme: &Theme, rect: Rect, state: &mut State) {
    let bar = Rect::from_min_max(
        pos2(rect.left(), rect.top() + 28.0),
        pos2(rect.right() - PAD, rect.top() + 54.0),
    );
    let layout = egui::Layout::right_to_left(egui::Align::Min);
    ui.scope_builder(egui::UiBuilder::new().max_rect(bar).layout(layout), |ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if button_secondary(ui, theme, "100%").clicked() {
            state.view.one_to_one(ui.ctx().pixels_per_point());
        }
        if button_secondary(ui, theme, "Encuadrar").clicked() {
            state.frame = true;
        }
        if state.scope == Scope::Piece {
            let tag = |p: &Painter, r: Rect, c| glyph::paint(p, r, c, TAG);
            if button_icon(ui, theme, "Etiquetas", state.labels, tag).clicked() {
                state.labels = !state.labels;
            }
        }
    });
}
