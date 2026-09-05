use toile_engine::draft::{Command, Identity, Point, PointKey, SegmentEdit, curve as cubic};

use super::super::curve;
use super::super::gesture::{EditContext, Feedback, Gesture, Stack};
use super::super::pick::Nearest;
use super::super::state::Selection;

/// The name an insertion leaves in the undo stack.
pub(super) const INSERT: &str = "insertar punto";

/// The name a deletion leaves.
pub(super) const REMOVE: &str = "borrar punto";

/// The Point tool on a tract: a node lands exactly where the line runs.
///
/// The place is not rounded — a vertex pulled to the nearest tenth would
/// leave the very line it was asked to sit on. On a straight tract the node
/// takes the nearest place on the segment; a bending one is cut in two, so
/// the drawn line does not move and the node count is what changes.
pub(super) fn insert(ctx: &EditContext<'_>, found: &Nearest) -> (Gesture, Vec<Command>, Feedback) {
    let node = ctx.tracts[found.from].node;
    let commands = match cut(ctx, node, found.at) {
        Some(halves) => halves,
        None => vec![Command::InsertNode {
            piece: ctx.piece,
            after: Some(node),
            identity: Identity::New,
            value: Point::at(found.at[0], found.at[1]),
            segment: SegmentEdit::Line,
            samples: 1,
        }],
    };
    (
        Gesture::Idle,
        commands,
        Feedback {
            stack: Some(Stack::Once(INSERT)),
            ..Feedback::default()
        },
    )
}

/// The two edits that cut a bending tract in two, when the tract bends.
///
/// De Casteljau: the tract that stays keeps its node and takes the first half
/// of the control net, and the new node opens the second, so the two halves
/// trace exactly the line the whole traced. Both keep the original count of
/// samples: a cut is not the place to coarsen a curve somebody tuned.
fn cut(ctx: &EditContext<'_>, node: PointKey, at: [f64; 2]) -> Option<Vec<Command>> {
    let bend = ctx.bends.iter().find(|bend| bend.node == node)?;
    let samples = curve::samples_of(ctx.doc, ctx.piece, node)?;
    let net = [bend.from, bend.out.1, bend.into.1, bend.to];
    // The Bezier parameter of the nearest curve point, not the arc-length
    // fraction the pick reports: the two part company by millimetres on a
    // real contour, and de Casteljau cuts at the parameter.
    let (t, _) = cubic::nearest(net[0], net[1], net[2], net[3], at);
    let (first, second) = cubic::subdivide(net[0], net[1], net[2], net[3], t);
    Some(vec![
        Command::SetSegment {
            piece: ctx.piece,
            node,
            to: SegmentEdit::cubic(
                Point::at(first[1][0], first[1][1]),
                Point::at(first[2][0], first[2][1]),
            ),
        },
        Command::InsertNode {
            piece: ctx.piece,
            after: Some(node),
            identity: Identity::New,
            value: Point::at(second[0][0], second[0][1]),
            segment: SegmentEdit::cubic(
                Point::at(second[1][0], second[1][1]),
                Point::at(second[2][0], second[2][1]),
            ),
            samples,
        },
    ])
}

/// Supr on what is chosen: each chosen node leaves the contour, by key.
///
/// Only the contour's own nodes go — a chosen handle is not a node, and the
/// way to take a handle away is to straighten its tract. A node the document
/// refuses to give up, such as one another piece still draws itself with,
/// comes back as a refusal for the status bar, never as a panic.
pub(super) fn remove(ctx: &EditContext<'_>) -> (Gesture, Vec<Command>, Feedback) {
    let commands: Vec<Command> = ctx
        .selection
        .points()
        .filter(|&key| ctx.nodes.iter().any(|&(node, _)| node == key))
        .map(|node| Command::RemoveNode {
            piece: ctx.piece,
            node,
        })
        .collect();
    if commands.is_empty() {
        return (Gesture::Idle, Vec::new(), Feedback::default());
    }
    (
        Gesture::Idle,
        commands,
        Feedback {
            stack: Some(Stack::Once(REMOVE)),
            select: Some(Selection::None),
            ..Feedback::default()
        },
    )
}
