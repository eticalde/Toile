use eframe::egui::{Key, Pos2};
use toile_engine::draft::{Command, EdgeAnchor, NotchKey};

use super::gesture::{EditContext, Feedback, Gesture, Input, Stack};
use super::inner::Tick;
use super::state::Selection;
use super::{pick, tract};

/// The name a notch slid leaves in the undo stack.
pub const SLIDE: &str = "mover piquete";

/// How far the pointer has to travel before a press becomes a drag, in screen
/// points. Under it the gesture is a click, and a click edits nothing.
const HAIR: f32 = 2.0;

/// How finely a notch's fraction is written: ten thousandths of its tract.
///
/// The side of a trouser leg is about a metre, so one step of this is a tenth
/// of a millimetre along it — the resolution a drag already writes a coordinate
/// at. Without it the file would carry the whole of a pointer's arithmetic, and
/// a mark nobody moved would diff against itself.
const STEPS: f64 = 10_000.0;

/// A notch in hand, sliding along the tract it was cut into.
///
/// Only the notch travels: which tract it sits on and which piece that tract
/// belongs to are read back from the document on every frame, so the gesture
/// carries no copy of them to fall out of step with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slide {
    /// The notch.
    pub notch: NotchKey,
    /// Where the pointer was pressed, on the glass.
    pub grab: Pos2,
    /// Whether the pointer ever really left: a click is not a drag.
    pub moved: bool,
}

/// The notch a press lands on, when it lands on one.
///
/// A mark is a deliberate target, so it is caught within the budget a node is
/// caught within, and the nearest one wins when two marks share a corner.
pub fn under(at: [f64; 2], ticks: &[Tick], reach: f64) -> Option<NotchKey> {
    nearest(at, ticks, reach, |_| true)
}

/// The notch a press lands on while the tract it was cut into is the tract
/// already chosen.
///
/// This is the one reading that reaches a mark cut at a node, and a mark that
/// came in from Seamly always is: there a notch is written on an outline node,
/// so its fraction is zero and the mat draws the cut centred on the node's own
/// dot — the whole of the mark lies inside the budget the node is caught
/// within, and no press can tell the two apart by where it landed. The node
/// keeps the press: it is drawn over the mark, and it is the cut line. The mark
/// answers once somebody has said which tract they are working on, which is the
/// answer a place of the chosen line already gives.
pub fn on_chosen(at: [f64; 2], ctx: &EditContext<'_>, reach: f64) -> Option<NotchKey> {
    let from = ctx.selection.edge()?;
    nearest(at, ctx.ticks, reach, |notch| {
        ctx.doc
            .notches
            .get(notch)
            .is_some_and(|held| held.at.from == from)
    })
}

/// The nearest mark within `reach` out of those `keep` answers for.
fn nearest(
    at: [f64; 2],
    ticks: &[Tick],
    reach: f64,
    keep: impl Fn(NotchKey) -> bool,
) -> Option<NotchKey> {
    let mut best: Option<(f64, NotchKey)> = None;
    for tick in ticks {
        let away = pick::away(tick.at, at);
        if away < reach && best.is_none_or(|(kept, _)| away < kept) && keep(tick.notch) {
            best = Some((away, tick.notch));
        }
    }
    best.map(|(_, notch)| notch)
}

/// A press on a notch: the mark comes into hand, and nothing is written yet.
///
/// The tract it was cut into is chosen with it, which is what a press on that
/// tract would have done anyway — and it is the panel's own way of showing how
/// far along the mark sits, so the number moves while the hand does.
pub fn grab(notch: NotchKey, at: Pos2, ctx: &EditContext<'_>) -> (Gesture, Vec<Command>, Feedback) {
    let slide = Slide {
        notch,
        grab: at,
        moved: false,
    };
    let tract = ctx.doc.notches.get(notch).map(|held| held.at.from);
    (
        Gesture::Sliding(slide),
        Vec::new(),
        Feedback {
            stack: Some(Stack::Open(SLIDE)),
            select: tract.map(Selection::Edge),
            ..Feedback::default()
        },
    )
}

/// Reduces one event against the notch in hand.
///
/// Pure, like every other gesture of the mat: the slide comes back as a command
/// for the caller to apply. One `MoveNotch` per frame, and they fold into the
/// one entry the press opened, because the field a later one writes is the
/// field the earlier one wrote.
pub fn update(
    slide: Slide,
    event: &Input,
    ctx: &EditContext<'_>,
) -> (Gesture, Vec<Command>, Feedback) {
    match event {
        Input::Move(at, _) => moved(slide, *at, ctx),
        Input::Up(..) => rest(Stack::Close),
        // An abandoned slide is refused, not undone: what the hand backed out
        // of has no business on the redo stack. One that never left is closed,
        // since cancelling an entry with nothing in it takes back the entry
        // before it instead.
        Input::Key(Key::Escape, _) => rest(if slide.moved {
            Stack::Cancel
        } else {
            Stack::Close
        }),
        Input::Down(..) | Input::Key(..) | Input::Text(_) => {
            (Gesture::Sliding(slide), Vec::new(), Feedback::default())
        }
    }
}

/// One frame of the slide: the mark follows the pointer along its own tract.
///
/// Along that tract and nowhere else. The fraction is measured by arc length
/// and clamped to the tract's two ends, which is what keeps a mark on the
/// stretch of contour it was cut into however far the pointer wanders. A mark
/// whose tract the contour no longer runs through has nowhere to go, so the
/// gesture ends rather than guessing at one.
fn moved(mut slide: Slide, at: Pos2, ctx: &EditContext<'_>) -> (Gesture, Vec<Command>, Feedback) {
    if !slide.moved && (at - slide.grab).length() <= HAIR {
        return (Gesture::Sliding(slide), Vec::new(), Feedback::default());
    }
    let Some(held) = ctx.doc.notches.get(slide.notch) else {
        return rest(Stack::Close);
    };
    let found = ctx.tracts.iter().find(|it| it.node == held.at.from);
    let Some(t) = found.and_then(|tract| tract::fraction(tract, ctx.view.to_document(at))) else {
        return rest(Stack::Close);
    };
    slide.moved = true;
    let to = EdgeAnchor {
        t: stepped(t),
        ..held.at
    };
    (
        Gesture::Sliding(slide),
        vec![Command::MoveNotch {
            notch: slide.notch,
            to,
        }],
        Feedback::default(),
    )
}

/// A fraction at the resolution a notch is written to.
fn stepped(t: f64) -> f64 {
    (t * STEPS).round() / STEPS
}

/// Back to looking, with the stack moving as it says.
fn rest(stack: Stack) -> (Gesture, Vec<Command>, Feedback) {
    (
        Gesture::Idle,
        Vec::new(),
        Feedback {
            stack: Some(stack),
            ..Feedback::default()
        },
    )
}

#[cfg(test)]
mod tests;
