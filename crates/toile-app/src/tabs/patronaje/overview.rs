use eframe::egui::{self, Align2, CursorIcon, FontId, Painter, Rect, Response, Stroke, vec2};
use toile_engine::draft::{Draft, PieceKey};

use super::gesture::Gesture;
use super::layout::{self, Laid};
use super::pick::EDGE_PT;
use super::state::State;
use super::wire::{self, Verb};
use super::{arrange, canvas};
use crate::theme::Theme;
use crate::widgets::fill;

const EMPTY: &str = "Producto sin piezas";
const FIRST: &str = "Dibuja la primera con «+ Pieza», en el panel Producto.";

/// The whole product on the mat: every piece at once, where it was placed or
/// where the overview lines it up, to be chosen, moved and opened.
///
/// Nothing here reaches a node. A press takes a piece in hand, and a double
/// click opens it on its own, which is where its nodes are edited.
pub fn show(
    ui: &mut egui::Ui,
    theme: &Theme,
    (resp, painter): (&Response, &Painter),
    draft: &Draft,
    state: &mut State,
    verbs: &mut Vec<Verb>,
) {
    let rect = resp.rect;
    let laid = layout::of(draft);
    canvas::frame_once(state, layout::bounds(&laid), rect);
    let over = |state: &State| {
        let reach = EDGE_PT / state.view.scale().max(f64::EPSILON);
        resp.hover_pos()
            .and_then(|at| layout::under(state.view.to_document(at), &laid, reach))
            .map(|it| it.piece)
    };
    if state.ask.is_none() {
        wire::view_keys(ui, resp, state);
        reduce(ui, resp, &laid, state, verbs);
        if resp.double_clicked()
            && let Some(piece) = over(state)
        {
            state.open(piece);
        }
        grip(ui, state, over(state));
    }
    let over = over(state);
    fill(painter, theme, rect);
    canvas::mat_grid(painter, theme, rect, state.view);
    if laid.is_empty() {
        empty(painter, theme, rect);
    }
    for it in &laid {
        piece(painter, theme, draft, it, state, over);
    }
}

/// Runs this frame's events through the whole product's reducer, in the order
/// they happened.
fn reduce(ui: &egui::Ui, resp: &Response, laid: &[Laid], state: &mut State, verbs: &mut Vec<Verb>) {
    for event in wire::events_of(ui, resp) {
        let held = std::mem::take(&mut state.gesture);
        let (next, commands, said) = arrange::update(held, event, laid, state.view);
        state.gesture = next;
        wire::stacked(said.stack, commands, verbs);
        if let Some(piece) = said.chosen {
            state.active = Some(piece);
        }
        state.view.pan(said.pan);
    }
}

/// The hand the pointer shows: closed while it carries a piece, open over one
/// it could take.
fn grip(ui: &egui::Ui, state: &State, over: Option<PieceKey>) {
    if matches!(state.gesture, Gesture::Arrange(_)) {
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
    } else if over.is_some() {
        ui.ctx().set_cursor_icon(CursorIcon::Grab);
    }
}

/// One piece with its name on it, lit when it is the piece in front and more
/// faintly while the pointer is over it.
fn piece(
    p: &Painter,
    theme: &Theme,
    draft: &Draft,
    it: &Laid,
    state: &State,
    over: Option<PieceKey>,
) {
    let chosen = state.active == Some(it.piece);
    let lit = chosen || over == Some(it.piece);
    let mut grounds = vec![theme.paper];
    if lit {
        grounds.push(
            theme
                .accent
                .gamma_multiply(if chosen { 0.16 } else { 0.07 }),
        );
    }
    let broken = !draft.defects(it.piece).is_empty();
    let ink = if broken {
        theme.alert
    } else if chosen {
        theme.accent
    } else {
        theme.outline
    };
    let width = if lit { 2.0 } else { 1.5 };
    let line = Stroke::new(width, ink);
    canvas::paper_and_outline(p, &it.outline, state.view, &grounds, line);
    let held = draft.doc().pieces.get(it.piece);
    let (Some(held), Some(bbox)) = (held, layout::bounds(std::slice::from_ref(it))) else {
        return;
    };
    let middle = [f64::from(bbox.center().x), f64::from(bbox.center().y)];
    let ink = if lit { theme.ink } else { theme.ink_soft };
    p.text(
        state.view.to_screen(middle),
        Align2::CENTER_CENTER,
        &held.name,
        FontId::proportional(13.0),
        ink,
    );
}

/// A product with no pieces: nothing to show yet, and where the first one
/// comes from.
fn empty(p: &Painter, theme: &Theme, rect: Rect) {
    p.text(
        rect.center() - vec2(0.0, 12.0),
        Align2::CENTER_CENTER,
        EMPTY,
        FontId::proportional(17.0),
        theme.ink,
    );
    p.text(
        rect.center() + vec2(0.0, 12.0),
        Align2::CENTER_CENTER,
        FIRST,
        FontId::proportional(12.0),
        theme.muted,
    );
}
