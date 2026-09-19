use eframe::egui::Key;
use toile_engine::draft::{Command, Doc, EdgeRange, Seam, SeamKey, SeamOrientation};

use super::arrange::Arranged;
use super::gesture::{Gesture, Input, Stack};
use super::pick::{EDGE_PT, nearest_on};
use super::sew::{self, Spread};
use super::state::{Selection, Tool};
use super::view::View;

/// The name a seam taken out leaves in the undo stack, from the panel's press
/// and from the key alike.
pub const UNPICK: &str = "descoser";

/// One seam as the whole product draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct Thread {
    /// The seam.
    pub seam: SeamKey,
    /// The number the panel lists it under.
    pub ordinal: usize,
    /// Its two sides as lines on the mat, each in the direction the pairing
    /// walks it.
    pub sides: [Vec<[f64; 2]>; 2],
}

/// What the reducer reads of the mat besides the event.
#[derive(Debug, Clone, Copy)]
pub struct Reach<'a> {
    /// Every seam the mat draws.
    pub threads: &'a [Thread],
    /// Where the document lies on the glass.
    pub view: View,
    /// The tool in hand.
    pub tool: Tool,
    /// The seam chosen, when one is.
    pub chosen: Option<SeamKey>,
}

/// Every seam of the product that has two sides to draw, in key order.
///
/// A seam with a side of no length, or on a piece that is not on the mat,
/// keeps its number and draws nothing: the panel says what is wrong with it.
pub fn of(doc: &Doc, spread: &[Spread]) -> Vec<Thread> {
    doc.seams
        .iter()
        .enumerate()
        .filter_map(|(index, (key, seam))| {
            Some(Thread {
                seam: key,
                ordinal: index + 1,
                sides: sides(spread, seam)?,
            })
        })
        .collect()
}

/// The two sides of a seam as lines on the mat, each in the direction the
/// pairing walks it: side A head to tail, side B the same way when the seam
/// is aligned and tail to head when it is opposed.
pub fn sides(spread: &[Spread], seam: &Seam) -> Option<[Vec<[f64; 2]>; 2]> {
    let of = |range: EdgeRange| {
        let piece = range.piece()?;
        let on = spread.iter().find(|it| it.piece == piece)?;
        let line = sew::stretch(&on.tracts, range);
        (line.len() >= 2).then_some(line)
    };
    let (a, mut b) = (of(seam.a)?, of(seam.b)?);
    if seam.orientation == SeamOrientation::Opposed {
        b.reverse();
    }
    Some([a, b])
}

/// The seam whose thread lies within `reach` centimetres of a place on the
/// mat, the nearest when several do; between two as near, the one drawn last.
pub fn under(at: [f64; 2], threads: &[Thread], reach: f64) -> Option<SeamKey> {
    let mut best: Option<(f64, SeamKey)> = None;
    for thread in threads {
        let pairs = thread.sides.iter().flat_map(|side| side.windows(2));
        let away = pairs
            .map(|pair| nearest_on(pair[0], pair[1], at, 0).away)
            .fold(f64::INFINITY, f64::min);
        if away < reach && best.is_none_or(|(kept, _)| away <= kept) {
            best = Some((away, thread.seam));
        }
    }
    best.map(|(_, seam)| seam)
}

/// What an event means to the seams themselves, when it means anything.
///
/// Asked before the reducer of whichever tool is in hand, and `None` hands the
/// event on to it. A press on a thread chooses its seam the way a press on its
/// row does, and lets go of it when it was the one chosen; the sewing tool
/// keeps its presses, which pick tracts and would land on the same lines. A
/// thread lies along an outline, so a sewn edge no longer takes its piece in
/// hand: the inside of the piece still does.
pub fn update(
    gesture: &Gesture,
    event: &Input,
    reach: &Reach<'_>,
) -> Option<(Gesture, Vec<Command>, Arranged)> {
    let chooses = |select: Selection| {
        let said = Arranged {
            select: Some(select),
            ..Arranged::default()
        };
        Some((Gesture::Idle, Vec::new(), said))
    };
    match (gesture, event, reach.chosen) {
        (_, Input::Down(at, mods), chosen) if reach.tool != Tool::Sew && !mods.space => {
            let within = EDGE_PT / reach.view.scale().max(f64::EPSILON);
            let found = under(reach.view.to_document(*at), reach.threads, within)?;
            chooses(if chosen == Some(found) {
                Selection::None
            } else {
                Selection::Seam(found)
            })
        }
        (Gesture::Idle, Input::Key(Key::Delete | Key::Backspace, _), Some(seam)) => {
            let said = Arranged {
                stack: Some(Stack::Once(UNPICK)),
                ..Arranged::default()
            };
            Some((Gesture::Idle, vec![Command::RemoveSeam { seam }], said))
        }
        (Gesture::Idle, Input::Key(Key::Escape, _), Some(_)) if reach.tool != Tool::Sew => {
            chooses(Selection::None)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests;
