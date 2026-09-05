use std::collections::BTreeSet;

use eframe::egui::{Key, Pos2};
use toile_engine::draft::{Command, PointKey};

use super::gesture::{self, EditContext, Feedback, Gesture, Input, Mods, Stack};
use super::pick::EDGE_PT;
use super::state::{Selection, Tool};
use super::tract;

mod bend;
mod drag;
mod draw;
mod node;
mod take;

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
        // Before the general press: while a piece is being drawn, every event
        // belongs to the drawing.
        (Gesture::Drawing { pending, rubber }, event) => draw::update(pending, rubber, &event, ctx),
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

/// A press: on a node it takes the selection in hand, on a handle it pulls a
/// tangent, on a straight tract with the Curve tool it bends one, on the mat
/// it sweeps a band, and with space held it slides the drawing instead.
fn press(at: Pos2, mods: Mods, ctx: &EditContext<'_>) -> (Gesture, Vec<Command>, Feedback) {
    if mods.space {
        return (Gesture::Pan { from: at }, Vec::new(), Feedback::default());
    }
    // The Line tool draws wherever it is pressed: a node under the pointer is
    // a snap candidate for the vertex, not something to take in hand.
    if ctx.tool == Tool::Line {
        return draw::start(at, mods, ctx);
    }
    if let Some(key) = take::node_at(at, ctx) {
        return take::grab(key, at, mods, ctx);
    }
    if let Some(key) = bend::handle_at(at, ctx) {
        return bend::grab(key, at, mods, ctx);
    }
    let cm = ctx.view.to_document(at);
    let found = tract::nearest(cm, ctx.tracts, &[]).filter(|it| it.away < reach(ctx, EDGE_PT));
    let Some(found) = found else {
        return (
            Gesture::Marquee { from: cm, to: cm },
            Vec::new(),
            Feedback {
                select: Some(Selection::None),
                ..Feedback::default()
            },
        );
    };
    let node = ctx.tracts[found.from].node;
    if ctx.tool == Tool::Point {
        return node::insert(ctx, &found);
    }
    if ctx.tool == Tool::Curve
        && let Some(bent) = bend::draw(ctx, node, &ctx.tracts[found.from])
    {
        return bent;
    }
    (
        Gesture::Marquee { from: cm, to: cm },
        Vec::new(),
        Feedback {
            select: Some(Selection::Edge(node)),
            ..Feedback::default()
        },
    )
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
    if matches!(key, Key::Delete | Key::Backspace) {
        return node::remove(ctx);
    }
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
        (Key::V, false, _) => tool(Tool::Select),
        (Key::P, false, _) => tool(Tool::Point),
        (Key::L, false, _) => tool(Tool::Line),
        (Key::C, false, _) => tool(Tool::Curve),
        _ => Feedback::default(),
    };
    (Gesture::Idle, Vec::new(), feedback)
}

/// Putting a tool in hand, which chooses nothing and edits nothing.
fn tool(chosen: Tool) -> Feedback {
    Feedback {
        tool: Some(chosen),
        ..Feedback::default()
    }
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
mod bending;
#[cfg(test)]
mod curving;
#[cfg(test)]
mod drawing;
#[cfg(test)]
mod pointing;
#[cfg(test)]
mod select;
#[cfg(test)]
mod square;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod typing;
