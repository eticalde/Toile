use eframe::egui::Pos2;
use toile_engine::draft::{Command, PointKey};

use super::super::curve;
use super::super::gesture::{self, Drag, EditContext, Feedback, Follow, Gesture, Mods, Stack};
use super::super::pick::{self, NODE_PT};
use super::super::state::Selection;
use super::super::tract::Tract;
use super::reach;
use super::take::held;

/// The name a handle drag leaves in the undo stack.
const MOVE: &str = "mover manija";

/// The name the Curve tool leaves.
const DRAW: &str = "curvar borde";

/// The handle a press lands on, out of the ones the drawing is showing.
///
/// Only those: a handle nobody can see is a handle nobody meant to grab, and
/// the tangents of a whole piece lie close enough to its outline that catching
/// an invisible one would feel like the drawing moving on its own.
pub(super) fn handle_at(at: Pos2, ctx: &EditContext<'_>) -> Option<PointKey> {
    let cm = ctx.view.to_document(at);
    let shown = curve::handles(ctx.bends, &ctx.selection);
    pick::nearest_node(cm, &shown, &[], reach(ctx, NODE_PT)).map(|(key, _)| key)
}

/// A press on a handle: it comes into hand, and its mate across the node comes
/// with it the other way.
///
/// The mate is in hand even when `Alt` is already down, because the gesture
/// has to go on knowing which point it is refusing to move.
pub(super) fn grab(
    key: PointKey,
    at: Pos2,
    mods: Mods,
    ctx: &EditContext<'_>,
) -> (Gesture, Vec<Command>, Feedback) {
    let Some(anchor) = held(ctx, key, Follow::Along) else {
        return (Gesture::Idle, Vec::new(), Feedback::default());
    };
    let from = anchor.from;
    let mut nodes = vec![anchor];
    if let Some(hangs) = curve::hangs(ctx.doc, ctx.piece, key)
        && let Some(mate) = hangs.mate
        && let Some(other) = held(ctx, mate, Follow::Against)
    {
        nodes.push(other);
    }
    let drag = Drag {
        nodes,
        grab: at,
        to: from,
        moved: false,
        step: ctx.snap.step_cm(),
        free: mods.alt,
        typed: None,
    };
    (
        gesture::holding(drag),
        Vec::new(),
        Feedback {
            select: Some(Selection::point(key)),
            stack: Some(Stack::Open(MOVE)),
            ..Feedback::default()
        },
    )
}

/// The Curve tool on a straight tract: it gains two handles and a sample
/// count, in one entry of the history.
///
/// A tract that already bends is left alone and falls through to being chosen,
/// which is how its sample count is reached from the inspector.
pub(super) fn draw(
    ctx: &EditContext<'_>,
    node: PointKey,
    tract: &Tract,
) -> Option<(Gesture, Vec<Command>, Feedback)> {
    if curve::samples_of(ctx.doc, ctx.piece, node).is_some() {
        return None;
    }
    let ends = (*tract.line.first()?, *tract.line.last()?);
    let commands = curve::bend(ctx.doc, ctx.piece, node, ends);
    if commands.is_empty() {
        return None;
    }
    Some((
        Gesture::Idle,
        commands,
        Feedback {
            select: Some(Selection::Edge(node)),
            stack: Some(Stack::Once(DRAW)),
            ..Feedback::default()
        },
    ))
}
