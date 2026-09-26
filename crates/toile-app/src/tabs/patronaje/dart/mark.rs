use eframe::egui::{Painter, Pos2, Shape, Stroke, Vec2};
use toile_engine::draft::FoldDirection;

use super::super::view::View;
use super::declare::Declaring;
use super::{Cut, Darting};
use crate::theme::Theme;

/// How long each stroke of the cross at a dart's point is, in screen points.
const CROSS: f32 = 11.0;

/// How long the barb of the arrow that says which way it is pressed is.
const BARB: f32 = 7.0;

/// How far inside the cloth that arrow is drawn, in screen points.
///
/// Both legs of a wedge sit on one straight tract, so the mouth lies along the
/// very edge the wedge was taken out of. The arrow is set in toward the apex
/// and barbed on that side, so none of it is drawn over paper the cutter takes
/// off.
const INSET: f32 = 4.0;

/// The dash and the gap of the line across a dart's mouth.
const DASH: (f32, f32) = (5.0, 3.0);

/// How big the dot on a leg the hand has pressed is.
const DOT: f32 = 3.0;

/// The marks of every dart the piece carries, drawn over its paper.
///
/// In screen points and not in the centimetres the sheet measures the same
/// marks in, which is the notches' own answer: a mark on the glass has to stay
/// legible at every zoom, and a mark on paper is cut at true scale.
///
/// The cut line already draws the wedge, so what these add is the one thing it
/// cannot say: the mouth closes. It is drawn broken in the ink of a guide,
/// because nothing is cut along it.
pub fn drawn(p: &Painter, theme: &Theme, cuts: &[Cut], view: View) {
    let ink = Stroke::new(1.4, theme.measure);
    for cut in cuts {
        let Some(at) = cut.at else {
            continue;
        };
        let [first, apex, second] = at.map(|cm| view.to_screen(cm));
        let Some(along) = unit(first, second) else {
            continue;
        };
        p.extend(Shape::dashed_line(&[first, second], ink, DASH.0, DASH.1));
        let (tip, into) = pressed(cut.fold, (first, second), along);
        arrow(
            p,
            ink,
            (tip, first.lerp(second, 0.5)),
            (into, toward(along, first, apex)),
        );
        cross(p, ink, apex, along);
    }
}

/// The wedge being cut, between the presses that make it.
///
/// In the ink of a gesture and not of a mark, because until the apex goes down
/// it is not a dart of the document. The arrow is drawn from the second press
/// onward: which way the wedge will be pressed follows from which leg went down
/// first, and this is where that is read before it is written.
pub fn cutting(p: &Painter, theme: &Theme, view: View, darting: &Darting) {
    let ink = Stroke::new(1.5, theme.accent);
    let guide = Stroke::new(1.0, theme.accent.gamma_multiply(0.55));
    let rubber = view.to_screen(darting.rubber);
    match darting.ordered() {
        Some((first, second)) => {
            let (a, b) = (view.to_screen(first.cm), view.to_screen(second.cm));
            p.add(Shape::line(vec![a, rubber, b], guide));
            p.extend(Shape::dashed_line(&[a, b], ink, DASH.0, DASH.1));
            if let Some(along) = unit(a, b) {
                let (tip, into) = pressed(darting.fold(), (a, b), along);
                arrow(
                    p,
                    ink,
                    (tip, a.lerp(b, 0.5)),
                    (into, toward(along, a, rubber)),
                );
                cross(p, ink, rubber, along);
            }
        }
        None => {
            if let Some(leg) = darting.legs.first() {
                p.line_segment([view.to_screen(leg.cm), rubber], guide);
            }
        }
    }
    for leg in &darting.legs {
        p.circle_filled(view.to_screen(leg.cm), DOT, theme.accent);
    }
}

/// The wedge being declared, between the presses that name it.
///
/// The three nodes are drawn already, so what this adds is which of them the
/// hand has taken: a dot on each, and the run from the last of them to the
/// pointer. The mouth and the arrow wait for the third press, because until
/// then which node is the far leg is the pointer's to say and not the mat's.
pub fn declaring(p: &Painter, theme: &Theme, view: View, declaring: &Declaring) {
    let guide = Stroke::new(1.0, theme.accent.gamma_multiply(0.55));
    let mut walk: Vec<Pos2> = declaring
        .nodes
        .iter()
        .map(|&(_, cm)| view.to_screen(cm))
        .collect();
    walk.push(view.to_screen(declaring.rubber));
    p.add(Shape::line(walk, guide));
    for &(_, cm) in &declaring.nodes {
        p.circle_filled(view.to_screen(cm), DOT, theme.accent);
    }
}

/// The leg the folded wedge lies against, and the way the arrow runs into it.
fn pressed(fold: FoldDirection, legs: (Pos2, Pos2), along: Vec2) -> (Pos2, Vec2) {
    match fold {
        FoldDirection::TowardStart => (legs.0, -along),
        FoldDirection::TowardEnd => (legs.1, along),
    }
}

/// The arrow from the middle of the mouth to that leg, set into the cloth and
/// barbed on that side.
fn arrow(p: &Painter, ink: Stroke, run: (Pos2, Pos2), way: (Vec2, Vec2)) {
    let (into, toward) = way;
    let head = run.0 + toward * INSET;
    let barb = head - into * BARB + toward * BARB;
    p.add(Shape::line(vec![run.1 + toward * INSET, head, barb], ink));
}

/// Which side of the mouth the cloth is on: the quarter turn of the mouth that
/// points at the apex.
fn toward(along: Vec2, leg: Pos2, apex: Pos2) -> Vec2 {
    let across = along.rot90();
    if (apex - leg).dot(across) < 0.0 {
        -across
    } else {
        across
    }
}

/// The cross at the apex: where the stitching stops, named from both sides of
/// it, the way every count of notch names one place.
fn cross(p: &Painter, ink: Stroke, apex: Pos2, along: Vec2) {
    for direction in [along, along.rot90()] {
        let end = |side: f32| apex + direction * (side * CROSS / 2.0);
        p.line_segment([end(-1.0), end(1.0)], ink);
    }
}

/// The direction from one place on the glass to another, and nothing when they
/// are one place.
fn unit(from: Pos2, to: Pos2) -> Option<Vec2> {
    let run = to - from;
    (run.length() > f32::EPSILON).then(|| run.normalized())
}
