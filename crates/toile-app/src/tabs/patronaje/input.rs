use std::collections::BTreeSet;

use eframe::egui::{Key, Pos2};
use toile_engine::draft::{Command, LineKey, PointKey};

use super::gesture::{self, EditContext, Feedback, Gesture, Input, Mods, Stack};
use super::pick::EDGE_PT;
use super::state::{Selection, Tool};
use super::{inner, trace, tract};

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
        (
            Gesture::Drawing {
                pending,
                rubber,
                back_to,
            },
            event,
        ) => draw::update(pending, rubber, back_to, &event, ctx),
        // And while a line is being traced, so does every event of that.
        (Gesture::Tracing(held), event) => trace::update(held, &event, ctx),
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

/// A press: with the Line tool it opens a tracing, on a node it takes the
/// selection in hand, on a handle it pulls a tangent, on an internal line it
/// chooses that line, on a straight tract with the Curve tool it bends one, on
/// the mat it sweeps a band, and with space held it slides the drawing instead.
fn press(at: Pos2, mods: Mods, ctx: &EditContext<'_>) -> (Gesture, Vec<Command>, Feedback) {
    if mods.space {
        return (Gesture::Pan { from: at }, Vec::new(), Feedback::default());
    }
    if ctx.tool == Tool::Trace {
        return trace::begin(at, ctx);
    }
    // A press never begins a piece: drawing runs only inside the `Drawing`
    // gesture, which "+ Pieza" opens deliberately. A press here takes what is
    // already drawn in hand, or sweeps a band, or does nothing.
    if let Some(key) = take::node_at(at, ctx) {
        return take::grab(key, at, mods, ctx);
    }
    if let Some(key) = bend::handle_at(at, ctx) {
        return bend::grab(key, at, mods, ctx);
    }
    let cm = ctx.view.to_document(at);
    // After the nodes and the handles, and before the contour: an internal line
    // is drawn over the paper, so a press on one is aimed at it rather than at
    // whatever tract happens to run beneath. Only the choosing tool answers it,
    // the way only the sewing tool answers a press on a thread — a press with
    // Punto or Curva in hand is aimed at the contour.
    if ctx.tool == Tool::Select
        && let Some(line) = inner::under(cm, ctx.lines, reach(ctx, EDGE_PT))
    {
        return chosen(line, ctx);
    }
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

/// A press on an internal line: it is chosen, and let go of when it was the
/// line already chosen, the way a press on a seam's thread reads.
fn chosen(line: LineKey, ctx: &EditContext<'_>) -> (Gesture, Vec<Command>, Feedback) {
    let select = if ctx.selection.line() == Some(line) {
        Selection::None
    } else {
        Selection::Line(line)
    };
    rest(Feedback {
        select: Some(select),
        ..Feedback::default()
    })
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
        // A chosen line is what Delete takes, and it is asked first: with a
        // line chosen no node is, so the other reading would delete
        // nothing at all.
        return match ctx.selection.line() {
            Some(line) => inner::erased(line),
            None => node::remove(ctx),
        };
    }
    let feedback = match (key, mods.command, mods.shift) {
        // With nothing chosen there is nothing on the piece left to let go
        // of, so Escape lets go of the piece: back to the whole product.
        (Key::Escape, _, _) if ctx.selection == Selection::None => Feedback {
            overview: true,
            ..Feedback::default()
        },
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
        (Key::C, false, _) => tool(Tool::Curve),
        (Key::L, false, _) => tool(Tool::Trace),
        // Sewing joins two pieces, so its key leaves this one for the whole
        // product, where both can be seen.
        (Key::S, false, _) => Feedback {
            overview: true,
            ..tool(Tool::Sew)
        },
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
pub(super) fn reach(ctx: &EditContext<'_>, budget: f64) -> f64 {
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
pub(super) mod tests;
#[cfg(test)]
mod typing;
