use eframe::egui::Pos2;
use toile_engine::draft::{Command, PointKey};

use super::super::gesture::{self, Drag, EditContext, Feedback, Follow, Gesture, Stack};
use super::take::held;

/// The name a place dragged leaves in the undo stack.
pub(super) const MOVE: &str = "mover lugar";

/// A press on a place of the chosen line: that place alone comes into hand.
///
/// Alone on purpose. A place is not a node: no tangent hangs from it, and the
/// selection holds the line rather than a set of points, so there is nothing
/// for it to bring along. The line stays the thing chosen, which is what lets
/// the panel go on naming the line whose place is moving and the next press
/// take hold of another of its places.
///
/// From there it is the ordinary drag: the same snap ladder, the same precision
/// box, the same question on release when the place was written as a formula.
/// A place off the contour is a point of the document, so it can be nothing
/// else.
pub(super) fn grab(
    key: PointKey,
    at: Pos2,
    ctx: &EditContext<'_>,
) -> (Gesture, Vec<Command>, Feedback) {
    let Some(place) = held(ctx, key, Follow::Along) else {
        return (Gesture::Idle, Vec::new(), Feedback::default());
    };
    let from = place.from;
    let drag = Drag {
        nodes: vec![place],
        grab: at,
        to: from,
        moved: false,
        step: ctx.snap.step_cm(),
        free: false,
        typed: None,
    };
    (
        gesture::holding(drag),
        Vec::new(),
        Feedback {
            stack: Some(Stack::Open(MOVE)),
            ..Feedback::default()
        },
    )
}
