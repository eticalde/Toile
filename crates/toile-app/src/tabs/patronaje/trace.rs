use eframe::egui::{Key, Pos2};
use toile_engine::draft::{Command, EdgeAnchor, Identity, LineEdit, LineKey, LineKind, LineVertex};

use super::gesture::{EditContext, Feedback, Gesture, Input, Stack};
use super::input::reach;
use super::pick::{self, EDGE_PT, NODE_PT};
use super::state::Selection;
use super::tract;

/// The name a line traced leaves in the undo stack.
pub const TRACE: &str = "trazar línea";

/// What a line is for until somebody says otherwise.
///
/// The one kind that promises nothing. A run just drawn has not been called a
/// fold, a topstitch or a pocket mouth, and drawing it as one would have the
/// cutter, the mesher and the sewer act on a word nobody said. The inspector's
/// own control is one press away from the line the tool just made.
const UNSAID: LineKind = LineKind::Reference;

/// The fewest places a line runs through.
const PLACES: usize = 2;

/// One place of a line being traced: where on the contour it is, and where that
/// falls on the mat.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Place {
    /// The place on the contour.
    pub at: EdgeAnchor,
    /// Where it lies, in centimetres.
    pub cm: [f64; 2],
}

/// The line being traced, between the presses that make it.
///
/// Nothing has reached the document yet, which is what lets Escape walk away
/// from any number of places with no entry to unwind, and Backspace take the
/// last one back for free.
#[derive(Debug, Clone, PartialEq)]
pub struct Tracing {
    /// The places pressed so far, in the order they were pressed.
    pub pending: Vec<Place>,
    /// Where the next place would land, in centimetres.
    pub rubber: [f64; 2],
}

/// Opens a tracing where a press landed, and opens none where it landed on
/// nothing.
///
/// A line is drawn on the cloth of one piece, so every place of it is a place
/// of that piece's contour: a press on the bare mat has nowhere to put one, and
/// says so by doing nothing at all.
pub fn begin(at: Pos2, ctx: &EditContext<'_>) -> (Gesture, Vec<Command>, Feedback) {
    match aim(at, ctx) {
        Some(place) => held(Tracing {
            pending: vec![place],
            rubber: place.cm,
        }),
        None => (Gesture::Idle, Vec::new(), Feedback::default()),
    }
}

/// Reduces one event against the line being traced.
///
/// Pure, like the reducer that draws a piece: the line comes back as a command
/// for the caller to apply, and no command goes out until the run is finished.
pub fn update(
    tracing: Tracing,
    event: &Input,
    ctx: &EditContext<'_>,
) -> (Gesture, Vec<Command>, Feedback) {
    match *event {
        Input::Down(_, mods) if mods.space => held(tracing),
        Input::Down(at, _) => pressed(tracing, at, ctx),
        Input::Move(at, _) => {
            let rubber = aim(at, ctx).map_or(ctx.view.to_document(at), |place| place.cm);
            held(Tracing { rubber, ..tracing })
        }
        Input::Key(Key::Enter, _) => close(&tracing, ctx),
        // Nothing of it reached the document, so there is nothing to unwind.
        Input::Key(Key::Escape, _) => (Gesture::Idle, Vec::new(), Feedback::default()),
        Input::Key(Key::Backspace | Key::Delete, _) => {
            let mut tracing = tracing;
            tracing.pending.pop();
            held(tracing)
        }
        Input::Up(..) | Input::Key(..) | Input::Text(_) => held(tracing),
    }
}

/// A press: on the place pressed last it finishes the line, and anywhere else
/// on the contour it carries the run on.
///
/// The finishing hit is asked of the raw pointer before the aim runs, so coming
/// back to the place just pressed ends the line instead of landing another
/// place a fraction along the tract beside it.
fn pressed(tracing: Tracing, at: Pos2, ctx: &EditContext<'_>) -> (Gesture, Vec<Command>, Feedback) {
    let cm = ctx.view.to_document(at);
    let last = tracing.pending.last().map(|place| place.cm);
    if tracing.pending.len() >= PLACES
        && last.is_some_and(|last| pick::away(cm, last) < reach(ctx, NODE_PT))
    {
        return close(&tracing, ctx);
    }
    let Some(place) = aim(at, ctx) else {
        return held(tracing);
    };
    let mut tracing = tracing;
    // A second press on one spot would write a span of no length, which the
    // document would refuse later and louder.
    if last != Some(place.cm) {
        tracing.pending.push(place);
    }
    tracing.rubber = place.cm;
    held(tracing)
}

/// The place on the contour a press lands on: a node when the pointer is near
/// one, and otherwise the place along the tract under it.
///
/// A node first, because that is what the drawing's own names are attached to
/// and what a person aims at; the ladder is the one the pointer already reports
/// against, so what is under the pointer and what the press takes cannot
/// disagree.
fn aim(at: Pos2, ctx: &EditContext<'_>) -> Option<Place> {
    let cm = ctx.view.to_document(at);
    if let Some((node, place)) = pick::nearest_node(cm, ctx.nodes, &[], reach(ctx, NODE_PT)) {
        return Some(Place {
            at: EdgeAnchor::at_node(ctx.piece, node),
            cm: place,
        });
    }
    let found = tract::nearest(cm, ctx.tracts, &[]).filter(|it| it.away < reach(ctx, EDGE_PT))?;
    Some(Place {
        at: EdgeAnchor {
            piece: ctx.piece,
            from: ctx.tracts[found.from].node,
            t: found.t,
        },
        cm: found.at,
    })
}

/// Finishes the line: the whole run goes out as one entry of the history.
///
/// The key the line will take is written down before the command is applied,
/// the way a drawn piece's is, so the inspector opens on the line the hand just
/// made and its kind is one press away. A run through fewer than two places is
/// no line, and asking to finish one leaves the tracing where it was.
fn close(tracing: &Tracing, ctx: &EditContext<'_>) -> (Gesture, Vec<Command>, Feedback) {
    let mut places = tracing.pending.iter();
    let Some(head) = places.next().filter(|_| tracing.pending.len() >= PLACES) else {
        return held(tracing.clone());
    };
    let mut edit = LineEdit::new(ctx.piece, UNSAID, LineVertex::Contour(head.at));
    for place in places {
        edit = edit.to(LineVertex::Contour(place.at));
    }
    let line = LineKey::new(ctx.doc.lines.issued(), 0);
    (
        Gesture::Idle,
        vec![Command::AddLine {
            identity: Identity::New,
            line: Box::new(edit),
        }],
        Feedback {
            stack: Some(Stack::Once(TRACE)),
            select: Some(Selection::Line(line)),
            ..Feedback::default()
        },
    )
}

/// The tracing as it stands, with nothing to say.
fn held(tracing: Tracing) -> (Gesture, Vec<Command>, Feedback) {
    (Gesture::Tracing(tracing), Vec::new(), Feedback::default())
}

#[cfg(test)]
pub(super) mod tests;
