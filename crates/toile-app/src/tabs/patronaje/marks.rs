use eframe::egui::{Align2, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, vec2};
use toile_engine::draft::{Draft, PieceKey};

use super::pick::Hover;
use super::snap::{SnapKind, Snapped};
use super::state::State;
use super::view::View;
use crate::theme::Theme;

/// How far the guide of an axis reaches past the pointer, in screen points.
const GUIDE: f32 = 60.0;

/// The length of one dash of a guide, and of the gap after it.
const DASH: f32 = 5.0;

/// The nodes, and the names of the ones asking to be read.
pub fn nodes(
    p: &Painter,
    theme: &Theme,
    draft: &Draft,
    piece: PieceKey,
    state: &State,
    over: Hover,
) {
    let doc = draft.doc();
    for &(key, at) in draft.points_cm(piece) {
        let (chosen, under) = (state.selection.holds(key), over == Hover::Node(key));
        let screen = state.view.to_screen(at);
        if chosen {
            p.circle_filled(screen, 5.0, theme.alert);
            p.circle_stroke(screen, 9.0, Stroke::new(1.0, theme.alert));
        } else if under {
            p.circle_filled(screen, 4.0, theme.ink);
        } else {
            p.circle_filled(screen, 3.0, theme.accent);
        }
        if !state.labels {
            continue;
        }
        // A name its author wrote belongs to the drawing; the automatic number
        // is only an answer to the pointer, or to a node marked to show one.
        let held = doc.points.get(key);
        let asked = held.is_some_and(|point| point.label_visible);
        let name = match held.and_then(|point| point.label.clone()) {
            Some(written) => written,
            None if asked || chosen || under => doc.label_of(piece, key).unwrap_or_default(),
            None => continue,
        };
        let font = FontId::monospace(10.0);
        let at = screen + vec2(9.0, -9.0);
        p.text(at, Align2::LEFT_BOTTOM, name, font, theme.ink_soft);
    }
}

/// What the pointer caught, drawn where it caught it.
///
/// A snap nobody can see is a snap nobody can trust, so every rung of the
/// ladder has its own mark: a ring on a node, a tick across a tract, the
/// dashed guide of an axis, a cross on the grid.
pub fn candidate(p: &Painter, theme: &Theme, view: View, snapped: Snapped, anchor: [f64; 2]) {
    let Some(kind) = snapped.kind else { return };
    let at = view.to_screen(snapped.at);
    let ink = Stroke::new(1.2, theme.measure);
    match kind {
        SnapKind::Node(_) => {
            p.circle_stroke(at, 8.0, ink);
        }
        SnapKind::Edge { .. } => {
            p.line_segment([at + vec2(-5.0, -5.0), at + vec2(5.0, 5.0)], ink);
            p.line_segment([at + vec2(-5.0, 5.0), at + vec2(5.0, -5.0)], ink);
        }
        SnapKind::Grid => {
            p.line_segment([at + vec2(-5.0, 0.0), at + vec2(5.0, 0.0)], ink);
            p.line_segment([at + vec2(0.0, -5.0), at + vec2(0.0, 5.0)], ink);
        }
        SnapKind::Axis => guide(p, ink, view.to_screen(anchor), at),
    }
}

/// The dashed line from where the gesture started to where it has got to.
fn guide(p: &Painter, ink: Stroke, from: Pos2, to: Pos2) {
    let run = to - from;
    let span = run.length();
    if span < f32::EPSILON {
        return;
    }
    let step = run / span;
    let mut along = 0.0;
    while along < span + GUIDE {
        let head = from + step * along;
        let tail = from + step * (along + DASH).min(span + GUIDE);
        p.line_segment([head, tail], ink);
        along += 2.0 * DASH;
    }
}

/// The rectangle a marquee is sweeping, while the pointer is drawing it.
pub fn band(p: &Painter, theme: &Theme, rect: Rect) {
    p.rect_filled(rect, 0.0, theme.accent.gamma_multiply(0.10));
    p.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0, theme.accent),
        StrokeKind::Inside,
    );
}
