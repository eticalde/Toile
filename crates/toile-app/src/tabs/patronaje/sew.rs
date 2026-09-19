mod run;

use eframe::egui::{Key, Pos2, Vec2};
pub use run::{APART, LAST, Pick, Spread, facing, holds, line_of, shifted, spread, stretch, under};
use toile_engine::draft::{Command, Identity, Seam, SeamKey};

use super::arrange::Arranged;
use super::gesture::{Gesture, Input, Mods, Stack};
use super::pick::EDGE_PT;
use super::state::Tool;
use super::view::View;

/// The name one seam sewn leaves in the undo stack.
pub const SEW: &str = "coser";

/// The name a tract added to a sewn side, or taken off one, leaves there.
pub const RESIDE: &str = "ajustar un lado de la costura";

/// The first side in hand, between the presses that make a seam.
///
/// Nothing has reached the document yet, which is what lets Escape, or a
/// second press on the same side, let go of it with no entry to unwind.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sewing {
    /// The run of tracts already picked.
    pub first: Pick,
    /// Where the pointer last was, while it slides the mat with a side held.
    pub pan: Option<Pos2>,
}

/// The whole product as one frame worked it out, lent to every event of it.
#[derive(Debug, Clone, Copy)]
pub struct Seen<'a> {
    /// Every piece's tracts where the overview drew them.
    pub spread: &'a [Spread],
    /// Where the document lies on the glass.
    pub view: View,
    /// The seam chosen, which is the one a shift-press lengthens while no
    /// side is in hand.
    pub chosen: Option<(SeamKey, Seam)>,
}

/// A seam taken out and put back under its own key as `to`, for the caller to
/// play as one entry.
///
/// The document has no edit that rewrites a seam, and it does not need one:
/// the key is what everything else knows the seam by, and it survives.
pub fn resewn(key: SeamKey, to: Seam) -> Vec<Command> {
    let back = Command::AddSeam {
        identity: Identity::Restored(key),
        seam: to,
    };
    vec![Command::RemoveSeam { seam: key }, back]
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
    seen: &Seen<'_>,
) -> (Gesture, Vec<Command>, Arranged) {
    match (gesture, event) {
        (held, Input::Down(at, mods)) => press(first_of(&held), at, mods, seen),
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

/// A press: on a tract it picks a side, and on the second side it sews; with
/// shift it adds the tract to the side in hand, or takes it off, and with no
/// side in hand does the same to the chosen seam; on the bare mat, or with
/// space held, it slides the product and keeps the side already picked.
fn press(
    first: Option<Pick>,
    at: Pos2,
    mods: Mods,
    seen: &Seen<'_>,
) -> (Gesture, Vec<Command>, Arranged) {
    let reach = EDGE_PT / seen.view.scale().max(f64::EPSILON);
    let found = under(seen.view.to_document(at), seen.spread, reach).filter(|_| !mods.space);
    let Some(second) = found else {
        let slide = first.map_or(Gesture::Pan { from: at }, |first| holding(first, Some(at)));
        return quiet(slide);
    };
    let chosen = Arranged {
        chosen: Some(second.piece),
        ..Arranged::default()
    };
    match (first, seen.chosen) {
        (Some(first), _) if mods.shift => match shifted(seen.spread, first, second) {
            Ok(Some(run)) => (holding(run, None), Vec::new(), chosen),
            Ok(None) => quiet(Gesture::Idle),
            Err(why) => refused(holding(first, None), why),
        },
        (None, Some((key, seam))) if mods.shift => resided(key, &seam, second, seen.spread),
        (None, _) => (holding(second, None), Vec::new(), chosen),
        (Some(first), _) => sewn(first, second, seen.spread, chosen),
    }
}

/// The second side pressed: the seam itself, as one entry.
///
/// A tract of the side already in hand puts that side down instead: a stretch
/// is not sewn to a part of itself, and the press has to mean something.
fn sewn(
    first: Pick,
    second: Pick,
    spread: &[Spread],
    chosen: Arranged,
) -> (Gesture, Vec<Command>, Arranged) {
    let lines = line_of(spread, first).zip(line_of(spread, second));
    let Some((a, b)) = lines.filter(|_| !holds(spread, first, second)) else {
        return quiet(Gesture::Idle);
    };
    let command = Command::AddSeam {
        identity: Identity::New,
        seam: Seam::plain(first.range(), second.range(), facing(&a, &b)),
    };
    let said = Arranged {
        stack: Some(Stack::Once(SEW)),
        ..chosen
    };
    (Gesture::Idle, vec![command], said)
}

/// A shift-press with no side in hand and a seam chosen: the tract goes onto
/// whichever side of that seam it continues, or comes off the end of the side
/// it belongs to, as one entry.
///
/// The side picked last is asked first, because that is the one the hand has
/// just left; which way the seam runs is the person's to say and is kept.
fn resided(
    key: SeamKey,
    seam: &Seam,
    tract: Pick,
    spread: &[Spread],
) -> (Gesture, Vec<Command>, Arranged) {
    let mut why = APART;
    for second in [true, false] {
        let range = if second { seam.b } else { seam.a };
        let Some(run) = Pick::of(range) else {
            continue;
        };
        let to = match shifted(spread, run, tract) {
            Ok(Some(run)) => run.range(),
            Ok(None) => return refused(Gesture::Idle, LAST),
            Err(apart) if apart == APART => continue,
            Err(other) => {
                why = other;
                break;
            }
        };
        let to = if second {
            Seam { b: to, ..*seam }
        } else {
            Seam { a: to, ..*seam }
        };
        let said = Arranged {
            stack: Some(Stack::Once(RESIDE)),
            ..Arranged::default()
        };
        return (Gesture::Idle, resewn(key, to), said);
    }
    refused(Gesture::Idle, why)
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

fn slid(gesture: Gesture, pan: Vec2) -> (Gesture, Vec<Command>, Arranged) {
    let said = Arranged {
        pan,
        ..Arranged::default()
    };
    (gesture, Vec::new(), said)
}

fn quiet(gesture: Gesture) -> (Gesture, Vec<Command>, Arranged) {
    (gesture, Vec::new(), Arranged::default())
}

/// Nothing done, and the reason said where the person will read it.
fn refused(gesture: Gesture, why: &'static str) -> (Gesture, Vec<Command>, Arranged) {
    let said = Arranged {
        refused: Some(why),
        ..Arranged::default()
    };
    (gesture, Vec::new(), said)
}

#[cfg(test)]
pub(super) mod tests;
