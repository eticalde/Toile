use std::collections::BTreeSet;

use eframe::egui::{Key, Pos2};
use toile_engine::draft::{Command, PointKey};

use super::gesture::{self, Drag, EditContext, Feedback, Gesture, Held, Input, Mods, Stack};
use super::pick::{self, EDGE_PT, NODE_PT};
use super::state::Selection;

mod drag;

/// The name a drag leaves in the undo stack.
const MOVE: &str = "mover punto";

/// Reduces one input event against the gesture in progress.
///
/// Pure: it reads the document, it never writes one. The commands come back
/// for the caller to apply, which is what lets a whole drag — grab, move with
/// snap, type an exact number, let go — be tested without opening a window.
pub fn update(
    gesture: Gesture,
    event: Input,
    ctx: &EditContext<'_>,
) -> (Gesture, Vec<Command>, Feedback) {
    match (gesture, event) {
        (_, Input::Down(at, mods)) => press(at, mods, ctx),
        (Gesture::Pan { from }, Input::Move(at, _)) => (
            Gesture::Pan { from: at },
            Vec::new(),
            Feedback {
                pan: at - from,
                ..Feedback::default()
            },
        ),
        (Gesture::Marquee { from, .. }, Input::Move(at, _)) => (
            Gesture::Marquee {
                from,
                to: ctx.view.to_document(at),
            },
            Vec::new(),
            Feedback::default(),
        ),
        (Gesture::Marquee { from, to }, Input::Up(..)) => swept(from, to, ctx),
        (Gesture::Drag(held), Input::Move(at, mods)) => drag::moved(*held, at, mods, ctx),
        (Gesture::Drag(held), Input::Up(..)) => drag::release(&held),
        (Gesture::Drag(held), Input::Text(text)) => drag::typing(*held, &text),
        (Gesture::Drag(held), Input::Key(key, mods)) => drag::during(*held, key, mods),
        (Gesture::Pan { .. }, Input::Up(..))
        | (Gesture::Marquee { .. }, Input::Key(Key::Escape, _)) => rest(Feedback::default()),
        (Gesture::Idle, Input::Key(key, mods)) => idle(key, mods, ctx),
        (held, _) => (held, Vec::new(), Feedback::default()),
    }
}

/// A press: on a node it takes the selection in hand, on the mat it sweeps
/// one, and with space held it slides the drawing instead.
fn press(at: Pos2, mods: Mods, ctx: &EditContext<'_>) -> (Gesture, Vec<Command>, Feedback) {
    if mods.space {
        return (Gesture::Pan { from: at }, Vec::new(), Feedback::default());
    }
    if let Some(key) = node_at(at, ctx) {
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
        return (
            gesture::holding(drag),
            Vec::new(),
            Feedback {
                select: Some(chosen),
                stack: Some(Stack::Open(MOVE)),
                ..Feedback::default()
            },
        );
    }
    let cm = ctx.view.to_document(at);
    let caught = pick::nearest_edge(cm, ctx.nodes, None)
        .filter(|found| found.away < reach(ctx, EDGE_PT))
        .map(|found| Selection::Edge(ctx.nodes[found.from].0));
    (
        Gesture::Marquee { from: cm, to: cm },
        Vec::new(),
        Feedback {
            select: Some(caught.unwrap_or(Selection::None)),
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
/// Nothing is in hand when the press left the node out of the selection, which
/// is what a shift-click that unchose it did.
fn hold(chosen: &Selection, key: PointKey, at: Pos2, ctx: &EditContext<'_>) -> Option<Drag> {
    if !chosen.holds(key) {
        return None;
    }
    let anchor = held_node(ctx, key)?;
    let from = anchor.from;
    let mut nodes = vec![anchor];
    nodes.extend(
        chosen
            .points()
            .filter(|&other| other != key)
            .filter_map(|other| held_node(ctx, other)),
    );
    Some(Drag {
        nodes,
        grab: at,
        to: from,
        moved: false,
        step: ctx.snap.step_cm(),
        typed: None,
    })
}

/// One node of the piece, with what it was bound to at this moment.
fn held_node(ctx: &EditContext<'_>, key: PointKey) -> Option<Held> {
    let point = ctx.doc.points.get(key)?;
    let &(_, from) = ctx.nodes.iter().find(|&&(other, _)| other == key)?;
    Some(Held {
        point: key,
        name: ctx.doc.label_of(ctx.piece, key).unwrap_or_default(),
        origin: [point.x.clone(), point.y.clone()],
        from,
    })
}

/// The node a press lands on, when it lands on one.
fn node_at(at: Pos2, ctx: &EditContext<'_>) -> Option<PointKey> {
    let cm = ctx.view.to_document(at);
    pick::nearest_node(cm, ctx.nodes, None, reach(ctx, NODE_PT)).map(|(key, _)| key)
}

/// Letting go of a marquee: every node inside the band is chosen.
///
/// A band that caught nothing changes nothing: the press has already said what
/// a click on bare mat means.
fn swept(from: [f64; 2], to: [f64; 2], ctx: &EditContext<'_>) -> (Gesture, Vec<Command>, Feedback) {
    let keys: BTreeSet<PointKey> = ctx
        .nodes
        .iter()
        .filter(|&&(_, at)| gesture::inside(from, to, at))
        .map(|&(key, _)| key)
        .collect();
    rest(Feedback {
        select: (!keys.is_empty()).then_some(Selection::Points(keys)),
        ..Feedback::default()
    })
}

/// A key pressed with nothing in hand.
fn idle(key: Key, mods: Mods, ctx: &EditContext<'_>) -> (Gesture, Vec<Command>, Feedback) {
    let feedback = match (key, mods.command, mods.shift) {
        (Key::Escape, _, _) => Feedback {
            select: Some(Selection::None),
            ..Feedback::default()
        },
        (Key::A, true, _) => Feedback {
            select: Some(Selection::Points(
                ctx.nodes.iter().map(|&(key, _)| key).collect(),
            )),
            ..Feedback::default()
        },
        (Key::Z, true, false) => Feedback {
            stack: Some(Stack::Undo),
            ..Feedback::default()
        },
        (Key::Z, true, true) => Feedback {
            stack: Some(Stack::Redo),
            ..Feedback::default()
        },
        _ => Feedback::default(),
    };
    (Gesture::Idle, Vec::new(), feedback)
}

/// Back to looking, carrying whatever the event left to say.
fn rest(feedback: Feedback) -> (Gesture, Vec<Command>, Feedback) {
    (Gesture::Idle, Vec::new(), feedback)
}

/// A budget in screen points, as a distance in centimetres.
fn reach(ctx: &EditContext<'_>, budget: f64) -> f64 {
    budget / ctx.view.scale().max(f64::EPSILON)
}

#[cfg(test)]
mod select;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod typing;
