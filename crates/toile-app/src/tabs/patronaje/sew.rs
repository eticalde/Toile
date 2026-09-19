use eframe::egui::{Key, Pos2};
use toile_engine::draft::{
    Command, Draft, EdgeRange, Identity, PieceKey, PointKey, Seam, SeamOrientation,
};

use super::arrange::Arranged;
use super::gesture::{Gesture, Input, Mods, Stack};
use super::layout::Laid;
use super::pick::{EDGE_PT, away};
use super::state::Tool;
use super::tract::{self, Tract};
use super::view::View;

/// The name one seam sewn leaves in the undo stack.
pub const SEW: &str = "coser";

/// One piece's tracts where the whole product draws them, in centimetres of
/// the mat and not of the piece.
#[derive(Debug, Clone, PartialEq)]
pub struct Spread {
    /// The piece.
    pub piece: PieceKey,
    /// Its tracts in contour order, moved to where the overview laid it.
    pub tracts: Vec<Tract>,
}

/// One tract picked as a side of a seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pick {
    /// The piece the tract belongs to.
    pub piece: PieceKey,
    /// The node it leaves.
    pub from: PointKey,
    /// The node it runs to.
    pub to: PointKey,
}

/// The first side in hand, between the two presses that make a seam.
///
/// Nothing has reached the document yet, which is what lets Escape, or a
/// second press on the same tract, let go of it with no entry to unwind.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sewing {
    /// The tract already picked.
    pub first: Pick,
    /// Where the pointer last was, while it slides the mat with a tract held.
    pub pan: Option<Pos2>,
}

impl Pick {
    /// The stretch of contour the pick names, node to node.
    pub fn range(self) -> EdgeRange {
        EdgeRange::between(self.piece, self.from, self.to)
    }
}

/// Every piece's tracts, moved to where the overview draws the piece.
pub fn spread(draft: &Draft, laid: &[Laid]) -> Vec<Spread> {
    laid.iter()
        .map(|it| {
            let mut tracts = tract::of(draft, it.piece);
            for at in tracts.iter_mut().flat_map(|tract| tract.line.iter_mut()) {
                *at = [at[0] + it.shift[0], at[1] + it.shift[1]];
            }
            Spread {
                piece: it.piece,
                tracts,
            }
        })
        .collect()
}

/// The tract a pick names, as the mat draws it now; nothing once its piece or
/// its node has gone.
pub fn tract_of(spread: &[Spread], pick: Pick) -> Option<&Tract> {
    let on = spread.iter().find(|it| it.piece == pick.piece)?;
    on.tracts
        .iter()
        .find(|tract| tract.node == pick.from && tract.to == pick.to)
}

/// The tract within `reach` centimetres of a place on the mat, the nearest
/// when several are; between two as near, the one drawn on top.
pub fn under(at: [f64; 2], spread: &[Spread], reach: f64) -> Option<Pick> {
    let mut best: Option<(f64, Pick)> = None;
    for it in spread {
        let Some(found) = tract::nearest(at, &it.tracts, &[]) else {
            continue;
        };
        if found.away < reach && best.is_none_or(|(kept, _)| found.away <= kept) {
            let tract = &it.tracts[found.from];
            let pick = Pick {
                piece: it.piece,
                from: tract.node,
                to: tract.to,
            };
            best = Some((found.away, pick));
        }
    }
    best.map(|(_, pick)| pick)
}

/// Which way round two tracts are sewn, from how they lie on the mat.
///
/// Pieces laid side by side the same way up are sewn top to top and bottom to
/// bottom, whichever way each contour happens to run past the seam. So the
/// ends that lie nearest each other are the ends that meet: when pairing head
/// with head is the shorter reach the sides run aligned, and otherwise they
/// run opposed. It is a first answer, and the inspector flips it.
pub fn facing(a: &Tract, b: &Tract) -> SeamOrientation {
    let ends = |tract: &Tract| Some((*tract.line.first()?, *tract.line.last()?));
    let (Some((head_a, tail_a)), Some((head_b, tail_b))) = (ends(a), ends(b)) else {
        return SeamOrientation::Aligned;
    };
    let aligned = away(head_a, head_b) + away(tail_a, tail_b);
    let opposed = away(head_a, tail_b) + away(tail_a, head_b);
    if opposed < aligned {
        SeamOrientation::Opposed
    } else {
        SeamOrientation::Aligned
    }
}

/// Reduces one input event against the whole product with the sewing tool in
/// hand.
///
/// Pure, like the reducer that arranges pieces: the seam comes back as a
/// command for the caller to apply. A seam is the one thing this tool edits,
/// so no piece moves under it and no node can be reached from here.
pub fn update(
    gesture: Gesture,
    event: Input,
    spread: &[Spread],
    view: View,
) -> (Gesture, Vec<Command>, Arranged) {
    match (gesture, event) {
        (held, Input::Down(at, mods)) => press(first_of(&held), at, mods, spread, view),
        (Gesture::Pan { from }, Input::Move(at, _)) => slid(Gesture::Pan { from: at }, at - from),
        (
            Gesture::Sewing(Sewing {
                first,
                pan: Some(from),
            }),
            Input::Move(at, _),
        ) => slid(holding(first, Some(at)), at - from),
        (Gesture::Sewing(held), Input::Up(..)) => quiet(holding(held.first, None)),
        (Gesture::Pan { .. }, Input::Up(..)) | (Gesture::Sewing(_), Input::Key(Key::Escape, _)) => {
            quiet(Gesture::Idle)
        }
        (Gesture::Idle, Input::Key(key, mods)) => idle(key, mods),
        (held, _) => quiet(held),
    }
}

/// A press: on a tract it picks a side, and on the second side it sews; on
/// the bare mat, or with space held, it slides the product and keeps the side
/// already picked.
fn press(
    first: Option<Pick>,
    at: Pos2,
    mods: Mods,
    spread: &[Spread],
    view: View,
) -> (Gesture, Vec<Command>, Arranged) {
    let reach = EDGE_PT / view.scale().max(f64::EPSILON);
    let found = under(view.to_document(at), spread, reach).filter(|_| !mods.space);
    let Some(second) = found else {
        let slide = first.map_or(Gesture::Pan { from: at }, |first| holding(first, Some(at)));
        return quiet(slide);
    };
    let chosen = Arranged {
        chosen: Some(second.piece),
        ..Arranged::default()
    };
    let Some(first) = first else {
        return (holding(second, None), Vec::new(), chosen);
    };
    // The same tract again puts it down: a tract is not sewn to itself, and
    // the press has to mean something.
    let sides = tract_of(spread, first).zip(tract_of(spread, second));
    let Some((a, b)) = sides.filter(|_| first != second) else {
        return quiet(Gesture::Idle);
    };
    let seam = Seam::plain(first.range(), second.range(), facing(a, b));
    let command = Command::AddSeam {
        identity: Identity::New,
        seam,
    };
    let said = Arranged {
        stack: Some(Stack::Once(SEW)),
        ..chosen
    };
    (Gesture::Idle, vec![command], said)
}

/// A key pressed with no side in hand.
///
/// Escape with nothing picked has only the tool left to let go of.
fn idle(key: Key, mods: Mods) -> (Gesture, Vec<Command>, Arranged) {
    let said = match (key, mods.command) {
        (Key::Z, true) => Arranged {
            stack: Some(if mods.shift { Stack::Redo } else { Stack::Undo }),
            ..Arranged::default()
        },
        (Key::Escape | Key::V, false) => Arranged {
            tool: Some(Tool::Select),
            ..Arranged::default()
        },
        _ => Arranged::default(),
    };
    (Gesture::Idle, Vec::new(), said)
}

/// The side already picked, out of whatever gesture the event found.
fn first_of(gesture: &Gesture) -> Option<Pick> {
    match gesture {
        Gesture::Sewing(held) => Some(held.first),
        _ => None,
    }
}

fn holding(first: Pick, pan: Option<Pos2>) -> Gesture {
    Gesture::Sewing(Sewing { first, pan })
}

fn slid(gesture: Gesture, pan: eframe::egui::Vec2) -> (Gesture, Vec<Command>, Arranged) {
    let said = Arranged {
        pan,
        ..Arranged::default()
    };
    (gesture, Vec::new(), said)
}

fn quiet(gesture: Gesture) -> (Gesture, Vec<Command>, Arranged) {
    (gesture, Vec::new(), Arranged::default())
}

#[cfg(test)]
mod tests;
