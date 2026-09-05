use eframe::egui::{Align2, FontId, Painter, Stroke, vec2};
use toile_engine::draft::{Draft, PieceKey, PointKey};

use super::pick::Hover;
use super::state::State;
use super::view::View;
use crate::theme::Theme;

/// How far a length sits off the tract it measures, in screen points.
const OFFSET: f32 = 11.0;

/// The shortest tract that still has room for its own number, on the glass.
const ROOM: f32 = 26.0;

/// The lengths the drawing is showing: the tract under the pointer, the one
/// chosen, the two a chosen node hangs from, and every one of them when the
/// measuring tape is out.
///
/// Measuring is never modal: the tract under the pointer always reads out, and
/// the tile only decides whether the rest of them do too.
pub fn show(
    p: &Painter,
    theme: &Theme,
    draft: &Draft,
    piece: PieceKey,
    state: &State,
    over: Hover,
) {
    let nodes = draft.points_cm(piece);
    if nodes.len() < 2 {
        return;
    }
    for (index, &(key, _)) in nodes.iter().enumerate() {
        if !shown(state, over, key) {
            continue;
        }
        tract(p, theme, draft, piece, state.view, index);
    }
}

/// Whether the tract leaving this node carries its length right now.
fn shown(state: &State, over: Hover, node: PointKey) -> bool {
    if state.dimensions || over == Hover::Edge(node) || state.selection.edge() == Some(node) {
        return true;
    }
    state.selection.holds(node)
}

/// One tract's length, written alongside it.
///
/// The number is the draft's own measurement of the run and not a distance
/// worked out again here, so the drawing and the status bar cannot disagree.
fn tract(p: &Painter, theme: &Theme, draft: &Draft, piece: PieceKey, view: View, index: usize) {
    let nodes = draft.points_cm(piece);
    let (from, a) = nodes[index];
    let (to, b) = nodes[(index + 1) % nodes.len()];
    let (head, tail) = (view.to_screen(a), view.to_screen(b));
    let run = tail - head;
    if run.length() < ROOM {
        return;
    }
    // The number sits on the outside of the run, turned the same way the
    // contour turns, so two tracts meeting at a node do not write over it.
    let off = vec2(run.y, -run.x) / run.length() * OFFSET;
    let at = head + run / 2.0 + off;
    let ink = theme.measure;
    p.line_segment(
        [head + off, tail + off],
        Stroke::new(0.8, ink.gamma_multiply(0.5)),
    );
    let text = format!("{:.1}", draft.run_length_cm(piece, from, to));
    p.text(
        at,
        Align2::CENTER_CENTER,
        text,
        FontId::monospace(10.0),
        ink,
    );
}
