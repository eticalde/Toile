use eframe::egui::{Key, Pos2, Vec2};
use toile_engine::draft::{Command, PieceKey, Placement};

use super::gesture::{Gesture, Input, Mods, Stack};
use super::layout::{self, Laid};
use super::pick::EDGE_PT;
use super::state::{Selection, Tool};
use super::view::View;

/// The name one drag of a piece leaves in the undo stack.
pub const MOVE: &str = "mover pieza";

/// How far a press may tremble, in screen points, and still be a click.
const HAIR: f32 = 2.0;

/// Placements are written in tenths of a centimetre, like a free drag.
const TENTHS: f64 = 10.0;

/// A piece taken in hand on the whole product.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arrange {
    /// The piece in hand.
    pub piece: PieceKey,
    /// Where the pointer was pressed, on the glass.
    pub grab: Pos2,
    /// How far the overview had moved the piece from its own coordinates when
    /// it was pressed, in centimetres.
    pub from: [f64; 2],
    /// Whether the pointer ever really left: a click is not a drag.
    pub moved: bool,
}

/// What one event on the whole product asks of the tab besides editing.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Arranged {
    /// Screen points the product slides by.
    pub pan: Vec2,
    /// The piece the event put in front, when it chose one.
    pub chosen: Option<PieceKey>,
    /// Where the undo stack moves.
    pub stack: Option<Stack>,
    /// The tool the event puts in hand, when it changes it.
    pub tool: Option<Tool>,
    /// The selection the event makes, when it makes one.
    pub select: Option<Selection>,
    /// What the event asked for and the mat would not do, in the words the
    /// status bar says it in.
    pub refused: Option<&'static str>,
}

/// Reduces one input event against the whole product.
///
/// Pure, like the reducer of a single piece: a move comes back as a placement
/// for the caller to apply. Where a piece sits is the one thing this view
/// edits, so no node, handle or formula can be reached from here.
pub fn update(
    gesture: Gesture,
    event: Input,
    laid: &[Laid],
    view: View,
) -> (Gesture, Vec<Command>, Arranged) {
    match (gesture, event) {
        (_, Input::Down(at, mods)) => press(at, mods, laid, view),
        (Gesture::Pan { from }, Input::Move(at, _)) => (
            Gesture::Pan { from: at },
            Vec::new(),
            Arranged {
                pan: at - from,
                ..Arranged::default()
            },
        ),
        (Gesture::Arrange(held), Input::Move(at, _)) => moved(held, at, view),
        (Gesture::Arrange(_), Input::Up(..)) => rest(Some(Stack::Close)),
        // A press that never left is closed, not cancelled: cancelling an
        // entry with nothing in it takes back the entry before it instead.
        (Gesture::Arrange(held), Input::Key(Key::Escape, _)) => rest(Some(if held.moved {
            Stack::Cancel
        } else {
            Stack::Close
        })),
        (Gesture::Pan { .. }, Input::Up(..)) => rest(None),
        (Gesture::Idle, Input::Key(Key::Z, mods)) if mods.command => {
            rest(Some(if mods.shift { Stack::Redo } else { Stack::Undo }))
        }
        (Gesture::Idle, Input::Key(Key::S, mods)) if !mods.command => (
            Gesture::Idle,
            Vec::new(),
            Arranged {
                tool: Some(Tool::Sew),
                ..Arranged::default()
            },
        ),
        (held, _) => (held, Vec::new(), Arranged::default()),
    }
}

/// A press: on a piece it takes the piece in hand and puts it in front; on the
/// bare mat, or with space held, it slides the whole product instead.
fn press(at: Pos2, mods: Mods, laid: &[Laid], view: View) -> (Gesture, Vec<Command>, Arranged) {
    let reach = EDGE_PT / view.scale().max(f64::EPSILON);
    let found = layout::under(view.to_document(at), laid, reach).filter(|_| !mods.space);
    let Some(found) = found else {
        return (Gesture::Pan { from: at }, Vec::new(), Arranged::default());
    };
    let held = Arrange {
        piece: found.piece,
        grab: at,
        from: found.shift,
        moved: false,
    };
    (
        Gesture::Arrange(held),
        Vec::new(),
        Arranged {
            chosen: Some(found.piece),
            stack: Some(Stack::Open(MOVE)),
            ..Arranged::default()
        },
    )
}

/// The piece following the pointer: placed where the overview drew it when it
/// was pressed, plus however far the pointer has gone since.
fn moved(mut held: Arrange, at: Pos2, view: View) -> (Gesture, Vec<Command>, Arranged) {
    if !held.moved && (at - held.grab).length() < HAIR {
        return (Gesture::Arrange(held), Vec::new(), Arranged::default());
    }
    held.moved = true;
    let (now, then) = (view.to_document(at), view.to_document(held.grab));
    let to = Placement::new(
        tenths(held.from[0] + now[0] - then[0]),
        tenths(held.from[1] + now[1] - then[1]),
    );
    let command = Command::PlacePiece {
        piece: held.piece,
        to: Some(to),
    };
    (Gesture::Arrange(held), vec![command], Arranged::default())
}

/// Back to looking, with the stack moving as it says.
fn rest(stack: Option<Stack>) -> (Gesture, Vec<Command>, Arranged) {
    (
        Gesture::Idle,
        Vec::new(),
        Arranged {
            stack,
            ..Arranged::default()
        },
    )
}

/// A placement coordinate, in tenths of a centimetre.
///
/// Adding zero turns a negative zero into a plain one. The file spells both
/// `0` and reads back the plain one, so without it the placement held here
/// would not be the very bits the reopened product holds.
fn tenths(cm: f64) -> f64 {
    (cm * TENTHS).round() / TENTHS + 0.0
}

#[cfg(test)]
mod tests;
