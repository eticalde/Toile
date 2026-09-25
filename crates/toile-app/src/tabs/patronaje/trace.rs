use eframe::egui::{Key, Pos2};
use toile_engine::draft::{
    Command, EdgeAnchor, Identity, LineEdit, LineKey, LineKind, Point, VertexEdit,
};

use super::gesture::{EditContext, Feedback, Gesture, Input, Mods, Stack};
use super::snap::{self, SnapConfig, SnapContext, SnapKind};
use super::state::Selection;
use super::{curve, tract};
use crate::bind;

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

/// Where one press of the Line tool put a place.
#[derive(Debug, Clone, PartialEq)]
pub enum Spot {
    /// On the contour of the piece, at a fraction of one of its tracts.
    On(EdgeAnchor),
    /// On the bare cloth of it, as the point the line will be drawn with.
    Loose(Point),
}

/// One place of a line being traced: what it is, and where that falls on the
/// mat.
#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    /// The place itself.
    pub at: Spot,
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

/// Opens a tracing where a press landed, and opens none where it landed off the
/// cloth.
///
/// A line is drawn on the cloth of one piece, so a press has to land on that
/// piece: on its contour, or inside the paper it is cut from. A press on the
/// bare mat beyond it has nowhere to put a place, and says so by doing nothing
/// at all.
pub fn begin(at: Pos2, mods: Mods, ctx: &EditContext<'_>) -> (Gesture, Vec<Command>, Feedback) {
    match aim(at, None, mods, ctx) {
        Some(place) => {
            let rubber = place.cm;
            held(Tracing {
                pending: vec![place],
                rubber,
            })
        }
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
    match event {
        Input::Down(_, mods) if mods.space => held(tracing),
        Input::Down(at, mods) => pressed(tracing, *at, *mods, ctx),
        Input::Move(at, mods) => {
            let rubber = aim(*at, Some(&tracing), *mods, ctx)
                .map_or(ctx.view.to_document(*at), |place| place.cm);
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
/// on the piece it carries the run on.
///
/// The finishing hit is asked of the raw pointer before the aim runs, so coming
/// back to the place just pressed ends the line instead of landing another
/// place a fraction along the tract beside it.
fn pressed(
    tracing: Tracing,
    at: Pos2,
    mods: Mods,
    ctx: &EditContext<'_>,
) -> (Gesture, Vec<Command>, Feedback) {
    let cm = ctx.view.to_document(at);
    let last = tracing.pending.last().map(|place| place.cm);
    if tracing.pending.len() >= PLACES
        && last.is_some_and(|last| super::pick::away(cm, last) < reach(ctx))
    {
        return close(&tracing, ctx);
    }
    let Some(place) = aim(at, Some(&tracing), mods, ctx) else {
        return held(tracing);
    };
    let mut tracing = tracing;
    tracing.rubber = place.cm;
    // A second press on one spot would write a span of no length, which the
    // document would refuse later and louder.
    if last != Some(place.cm) {
        tracing.pending.push(place);
    }
    held(tracing)
}

/// The place a press lands on, run through the mat's own snap ladder.
///
/// The ladder is the one every other gesture catches on — node, handle, tract,
/// axis, grid — so what the pointer reports and what the press takes cannot
/// disagree. A node or a tract makes a place on the contour, which follows the
/// cloth when the piece is re-drafted; anything else makes a place of its own
/// on the bare cloth. The handles on show are the chosen selection's, and a
/// tracing chooses nothing, so that rung stays quiet while a line is being
/// drawn.
fn aim(at: Pos2, tracing: Option<&Tracing>, mods: Mods, ctx: &EditContext<'_>) -> Option<Place> {
    let raw = ctx.view.to_document(at);
    let cfg = SnapConfig {
        on: ctx.snap.on && !mods.ctrl,
        axis: mods.shift,
        ..ctx.snap
    };
    let shown = curve::handles(ctx.bends, &ctx.selection);
    let anchor = tracing
        .and_then(|held| held.pending.last())
        .map_or(raw, |place| place.cm);
    let caught = snap::resolve(
        raw,
        &SnapContext {
            nodes: ctx.nodes,
            handles: &shown,
            tracts: ctx.tracts,
            held: &[],
            anchor,
            scale: ctx.view.scale().max(f64::EPSILON),
        },
        cfg,
    );
    let on = |anchor| {
        Some(Place {
            at: Spot::On(anchor),
            cm: caught.at,
        })
    };
    match caught.kind {
        Some(SnapKind::Node(node)) => on(EdgeAnchor::at_node(ctx.piece, node)),
        Some(SnapKind::Edge { from, t }) => on(EdgeAnchor {
            piece: ctx.piece,
            from,
            t,
        }),
        Some(SnapKind::Handle(_) | SnapKind::Axis | SnapKind::Grid) | None => {
            loose(caught.at, cfg.step_cm(), ctx)
        }
    }
}

/// The place a press makes on the bare cloth, and none at all off the cloth.
///
/// A mark has to go on meaning something when the pattern is re-drafted on
/// another body, and a pair of plain numbers does not: the paper moves and the
/// buttonhole stays behind. So the place takes the bindings of the node nearest
/// it and carries its offset from that node, which is the rule a drag already
/// writes by — [`bind::placed`] absorbs the offset into the adjustment term of
/// the formula it inherits, so a coordinate the node states as a measurement
/// stays a measurement and the mark travels with the cloth it was measured
/// from.
///
/// What it costs: the offset is in centimetres and it is frozen, so a
/// buttonhole a hand's width from the centre front is that far from it on every
/// body — which is what a buttonhole is — and a mark that has to hold a
/// proportion of a measurement is not what this press writes. The node is the
/// nearest one and nothing else, so a piece that re-drafts unevenly carries the
/// mark with that node alone.
fn loose(cm: [f64; 2], step: f64, ctx: &EditContext<'_>) -> Option<Place> {
    if !tract::covers(ctx.tracts, cm) {
        return None;
    }
    let (seat, from) = super::pick::nearest_node(cm, ctx.nodes, &[], f64::INFINITY)?;
    let held = ctx.doc.points.get(seat)?;
    let value = Point::at(
        bind::placed(&held.x, cm[0], cm[0] - from[0], step),
        bind::placed(&held.y, cm[1], cm[1] - from[1], step),
    );
    Some(Place {
        at: Spot::Loose(value),
        cm,
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
    let mut edit = LineEdit::new(ctx.piece, UNSAID, vertex(head));
    for place in places {
        edit = edit.to(vertex(place));
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

/// One pressed place as the drawing carries it into the document.
fn vertex(place: &Place) -> VertexEdit {
    match &place.at {
        Spot::On(anchor) => VertexEdit::Contour(*anchor),
        Spot::Loose(point) => VertexEdit::free(point.clone()),
    }
}

/// How near the pointer has to come back to the last place to end the line.
fn reach(ctx: &EditContext<'_>) -> f64 {
    super::input::reach(ctx, super::pick::NODE_PT)
}

/// The tracing as it stands, with nothing to say.
fn held(tracing: Tracing) -> (Gesture, Vec<Command>, Feedback) {
    (Gesture::Tracing(tracing), Vec::new(), Feedback::default())
}

#[cfg(test)]
mod cloth;
#[cfg(test)]
pub(super) mod tests;
