use eframe::egui::{self, Align2, CursorIcon, FontId, Painter, Rect, Response, Stroke, vec2};
use toile_engine::draft::{Draft, PieceKey, Seam, SeamKey};

use super::gesture::{Gesture, Input};
use super::layout::{self, Laid};
use super::pick::EDGE_PT;
use super::sew::{self, Pick, Spread};
use super::state::{State, Tool};
use super::thread::Thread;
use super::wire::{self, Verb};
use super::{arrange, canvas, chalk, fold, inner, marks, stitch, thread};
use crate::theme::Theme;
use crate::widgets::fill;

const EMPTY: &str = "Producto sin piezas";
const FIRST: &str = "Dibuja la primera con «+ Pieza», en el panel Producto.";

/// The whole product on the mat: every piece at once, where it was placed or
/// where the overview lines it up, to be chosen, moved and opened.
///
/// Nothing here reaches a node. A press takes a piece in hand, or with the
/// sewing tool a tract of one, and a double click opens a piece on its own,
/// which is where its nodes are edited.
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
    // Flattened once a frame, for the seams drawn along the tracts and for
    // the press that picks one.
    let spread = sew::spread(draft, &laid);
    let threads = thread::of(draft.doc(), &spread);
    canvas::frame_once(state, layout::bounds(&laid), rect);
    // A side picked on a piece that has since left the table names nothing.
    if let Gesture::Sewing(held) = &state.gesture
        && sew::line_of(&spread, held.first).is_none()
    {
        state.gesture = Gesture::Idle;
    }
    let over = |state: &State| {
        let reach = EDGE_PT / state.view.scale().max(f64::EPSILON);
        resp.hover_pos()
            .and_then(|at| layout::under(state.view.to_document(at), &laid, reach))
            .map(|it| it.piece)
    };
    if state.ask.is_none() {
        wire::view_keys(ui, resp, state);
        let seams = &draft.doc().seams;
        let chosen = state.selection.seam();
        let mat = Mat {
            laid: &laid,
            spread: &spread,
            threads: &threads,
            chosen: chosen.and_then(|key| Some((key, *seams.get(key)?))),
        };
        reduce(ui, resp, &mat, state, verbs);
        if resp.double_clicked()
            && let Some(piece) = over(state)
        {
            state.open(piece);
        }
        let reach = EDGE_PT / state.view.scale().max(f64::EPSILON);
        let pressable = aim(resp, &spread, state).is_some()
            || (state.tool != Tool::Sew
                && resp.hover_pos().is_some_and(|at| {
                    thread::under(state.view.to_document(at), &threads, reach).is_some()
                }));
        grip(ui, state, over(state), pressable);
    }
    let over = over(state);
    fill(painter, theme, rect);
    canvas::mat_grid(painter, theme, rect, state.view);
    if laid.is_empty() {
        empty(painter, theme, rect);
    }
    for (it, cut) in laid.iter().zip(&spread) {
        piece(painter, theme, draft, (it, cut), state, over);
    }
    let lit = (state.view, state.selection.seam());
    stitch::seams(painter, theme, &threads, lit);
    if state.tool == Tool::Sew {
        let first = match &state.gesture {
            Gesture::Sewing(held) => Some(held.first),
            _ => None,
        };
        let hands = (first, aim(resp, &spread, state));
        stitch::picking(painter, theme, &spread, state.view, hands);
    }
}

/// The tract a press would pick, while the sewing tool is in hand.
fn aim(resp: &Response, spread: &[Spread], state: &State) -> Option<Pick> {
    if state.tool != Tool::Sew {
        return None;
    }
    let reach = EDGE_PT / state.view.scale().max(f64::EPSILON);
    let at = state.view.to_document(resp.hover_pos()?);
    sew::under(at, spread, reach)
}

/// The whole product as one frame worked it out, lent to every event of it.
struct Mat<'a> {
    laid: &'a [Laid],
    spread: &'a [Spread],
    threads: &'a [Thread],
    /// The seam chosen as the frame began, as the document holds it.
    chosen: Option<(SeamKey, Seam)>,
}

/// Runs this frame's events through the whole product's reducers, in the order
/// they happened: the seams' own first, then the one that sews while that tool
/// is in hand or the one that arranges the pieces otherwise.
fn reduce(ui: &egui::Ui, resp: &Response, mat: &Mat<'_>, state: &mut State, verbs: &mut Vec<Verb>) {
    for event in wire::events_of(ui, resp) {
        let pressed = matches!(event, Input::Down(..));
        let held = std::mem::take(&mut state.gesture);
        // Narrowed per event: an earlier event of this frame may have let go
        // of the seam the frame began with.
        let chosen = mat
            .chosen
            .filter(|(key, _)| state.selection.seam() == Some(*key));
        let reach = thread::Reach {
            threads: mat.threads,
            view: state.view,
            tool: state.tool,
            chosen: chosen.map(|(key, _)| key),
        };
        let (next, commands, said) = match thread::update(&held, &event, &reach) {
            Some(answer) => answer,
            None if state.tool == Tool::Sew => {
                let seen = sew::Seen {
                    spread: mat.spread,
                    view: state.view,
                    chosen,
                };
                sew::update(held, event, &seen)
            }
            None => arrange::update(held, event, mat.laid, state.view),
        };
        state.gesture = next;
        wire::stacked(said.stack, commands, verbs);
        if let Some(piece) = said.chosen {
            state.active = Some(piece);
        }
        if let Some(tool) = said.tool {
            state.tool = tool;
        }
        if let Some(select) = said.select {
            state.choose(select);
        }
        // A press either is refused or is not, so it is the press that takes
        // the last refusal off the bar. The bar is drawn before the mat and a
        // refusal sends nothing to the sim, so nothing else would ask for the
        // frame that says it.
        if pressed || said.refused.is_some() {
            let now = said.refused.map(str::to_owned);
            if state.refused != now {
                ui.ctx().request_repaint();
            }
            state.refused = now;
        }
        state.view.pan(said.pan);
    }
}

/// The hand the pointer shows: closed while it carries a piece, a finger over
/// a line a press would pick — a tract with the sewing tool, a seam's thread
/// without it — and open over a piece it could take.
fn grip(ui: &egui::Ui, state: &State, over: Option<PieceKey>, pressable: bool) {
    if matches!(state.gesture, Gesture::Arrange(_)) {
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
    } else if pressable {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    } else if state.tool == Tool::Sew {
        // The sewing tool takes no piece in hand, so it shows no hand for one.
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
    (it, cut): (&Laid, &Spread),
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
    // The spread's tracts already lie where the overview put the piece.
    let at = (it.piece, cut.tracts.as_slice(), [0.0, 0.0]);
    marks::elastics(p, theme, draft.doc(), at, state.view);
    // The outline is the cloth, so a folded piece takes its real room here;
    // the crease over it is what says the drawing is half of that.
    canvas::paper_and_outline(p, &it.outline, state.view, &grounds, line);
    fold::crease(p, theme, (draft, it.piece), (state.view, it.shift));
    // The spread's tracts already lie where the overview put the piece, so only
    // a place bound to a free point of the document needs that offset.
    let lines = inner::of(draft, it.piece, &cut.tracts, it.shift);
    let ticks = inner::ticks(draft, it.piece, &cut.tracts);
    let ghosts = fold::mirrored(draft, it.piece, &lines, it.shift);
    let mirror = fold::reflected(draft, it.piece, &ticks, it.shift);
    chalk::lines(p, theme, &ghosts, (state.view, None));
    chalk::notches(p, theme, &mirror, state.view);
    chalk::lines(p, theme, &lines, (state.view, state.selection.line()));
    chalk::notches(p, theme, &ticks, state.view);
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
