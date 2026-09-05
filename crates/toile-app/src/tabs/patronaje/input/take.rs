use std::collections::BTreeSet;

use eframe::egui::Pos2;
use toile_engine::draft::{Command, PointKey};

use super::super::curve;
use super::super::gesture::{
    self, Drag, EditContext, Feedback, Follow, Gesture, Held, Mods, Stack,
};
use super::super::pick::{self, NODE_PT};
use super::super::state::Selection;
use super::{MOVE, reach};

/// What a handle with no name of its own is called in a question.
const HANDLE: &str = "manija";

/// A press on a node: it and whatever else is chosen come into hand.
pub(super) fn grab(
    key: PointKey,
    at: Pos2,
    mods: Mods,
    ctx: &EditContext<'_>,
) -> (Gesture, Vec<Command>, Feedback) {
    let chosen = choose(&ctx.selection, key, mods);
    let Some(drag) = hold(&chosen, key, at, ctx) else {
        // Shift took the node back out of the group: nothing is in hand.
        return (
            Gesture::Idle,
            Vec::new(),
            Feedback {
                select: Some(chosen),
                ..Feedback::default()
            },
        );
    };
    (
        gesture::holding(drag),
        Vec::new(),
        Feedback {
            select: Some(chosen),
            stack: Some(Stack::Open(MOVE)),
            ..Feedback::default()
        },
    )
}

/// What a press on `key` makes of the selection.
///
/// Shift adds the node to what was already chosen, and takes it back out when
/// it was already there. A plain press on a node the group already holds keeps
/// the whole group, so a selection is dragged without having to be made again.
fn choose(chosen: &Selection, key: PointKey, mods: Mods) -> Selection {
    let mut keys: BTreeSet<PointKey> = chosen.chosen().cloned().unwrap_or_default();
    if mods.shift {
        if !keys.insert(key) {
            keys.remove(&key);
        }
        return Selection::Points(keys);
    }
    if keys.contains(&key) {
        return Selection::Points(keys);
    }
    Selection::point(key)
}

/// The nodes a press takes in hand, the one under the pointer first.
///
/// The handles hanging from those nodes come too. A tangent is drawn from its
/// node, so a node moved without its handles would leave its own curve behind;
/// carrying them inside the same gesture is what keeps that one entry of the
/// history rather than three.
///
/// Nothing is in hand when the press left the node out of the selection, which
/// is what a shift-click that unchose it did.
fn hold(chosen: &Selection, key: PointKey, at: Pos2, ctx: &EditContext<'_>) -> Option<Drag> {
    if !chosen.holds(key) {
        return None;
    }
    let anchor = held(ctx, key, Follow::Along)?;
    let from = anchor.from;
    let mut nodes = vec![anchor];
    nodes.extend(
        chosen
            .points()
            .filter(|&other| other != key)
            .filter_map(|other| held(ctx, other, Follow::Along)),
    );
    // A point taken twice would be written twice and asked about twice, so a
    // handle the selection already holds is not carried a second time.
    let mut carried: Vec<PointKey> = Vec::new();
    for node in &nodes {
        for handle in curve::hanging(ctx.doc, ctx.piece, node.point) {
            let taken = carried.contains(&handle) || nodes.iter().any(|it| it.point == handle);
            if !taken {
                carried.push(handle);
            }
        }
    }
    nodes.extend(
        carried
            .into_iter()
            .filter_map(|handle| held(ctx, handle, Follow::Along)),
    );
    Some(Drag {
        nodes,
        grab: at,
        to: from,
        moved: false,
        step: ctx.snap.step_cm(),
        free: false,
        typed: None,
    })
}

/// One point of the piece, with what it was bound to at this moment.
pub(super) fn held(ctx: &EditContext<'_>, key: PointKey, follow: Follow) -> Option<Held> {
    let point = ctx.doc.points.get(key)?;
    let from = seat(ctx, key)?;
    Some(Held {
        point: key,
        name: name_of(ctx, key),
        origin: [point.x.clone(), point.y.clone()],
        from,
        follow,
    })
}

/// Where a point of the piece resolved to: a node of the contour, or a handle
/// of one of its tracts.
fn seat(ctx: &EditContext<'_>, key: PointKey) -> Option<[f64; 2]> {
    ctx.nodes
        .iter()
        .find_map(|&(other, at)| (other == key).then_some(at))
        .or_else(|| curve::at(ctx.bends, key))
}

/// What a point is called in the question a release may ask.
fn name_of(ctx: &EditContext<'_>, key: PointKey) -> String {
    ctx.doc
        .label_of(ctx.piece, key)
        .or_else(|| {
            ctx.doc
                .points
                .get(key)
                .and_then(|point| point.label.clone())
        })
        .unwrap_or_else(|| HANDLE.to_owned())
}

/// The node a press lands on, when it lands on one.
pub(super) fn node_at(at: Pos2, ctx: &EditContext<'_>) -> Option<PointKey> {
    let cm = ctx.view.to_document(at);
    pick::nearest_node(cm, ctx.nodes, &[], reach(ctx, NODE_PT)).map(|(key, _)| key)
}
