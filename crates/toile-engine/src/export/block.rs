/// How a block's lines are broken to the room it has, and how far apart they
/// then sit.
mod wrap;

use self::wrap::{stacked, wrapped};
use super::metric::{self, ABOVE, BELOW};
use super::{lands, units};

/// How many places across its own cloth a block tries besides its own corner.
///
/// Twelve each way, which across the widest sheet of the owner's jeans is a
/// step of about a centimetre: fine enough to find the tongue of cloth a tiled
/// sheet of them carries, and coarse enough that the whole search stays a few
/// hundred boxes. Doubling it moves no block of his jeans by a millimetre. A
/// block that stands wholly on its own cloth where it would rather be never
/// gets this far at all.
const STEPS: u32 = 12;

/// How much of a block's ink on its own cloth is enough to stop looking.
///
/// Not one, because the area of a box of type is worked out from widths in
/// thousandths of an em and a contour that was flattened from curves: asking
/// for the whole of it exactly would send every block on the sheet round the
/// full search to find out it was already home.
const HOME: f64 = 0.999;

/// How far inside its own box a block opens, in millimetres.
///
/// Clear of the piece's own cut line down the left of it, so the first letter
/// of every line is not standing on the line a person puts scissors on. It is
/// dropped where the room is too narrow to pay for it, because the words come
/// first.
const INSET: f64 = 1.5;

/// One line of a block, placed: what it says, the size it is set at, and where
/// its own left baseline goes, in millimetres with the second axis downward.
pub(super) struct Line {
    /// The size it is set at, in millimetres.
    pub(super) size: f64,
    /// What it says, whole.
    pub(super) body: String,
    /// Where its own left baseline goes.
    pub(super) at: [f64; 2],
}

/// The room a block has to find a place in.
pub(super) struct Room<'a> {
    /// The box the whole block must stay inside, in millimetres. On paper this
    /// is the part of the sheet that survives the trimming; in a drawing it is
    /// the drawing.
    pub(super) sheet: [f64; 4],
    /// The cut line of the piece whose words these are, as this sheet carries
    /// it. The cloth itself and not the box around it: the block is broken to
    /// the width of its own box, and then it looks for somewhere inside the
    /// closed line to stand, because the ink that ends up outside that line is
    /// ink the scissors throw away.
    pub(super) cloth: &'a [[f64; 2]],
    /// The boxes of everything already written here, which it may not touch.
    pub(super) taken: &'a [[f64; 4]],
    /// The outlines of the other pieces, which it may not enter.
    pub(super) others: &'a [&'a [[f64; 2]]],
}

/// Where each line of a block goes, or nothing when it has nowhere to go.
///
/// Four decisions in one place, because they are one decision. The block is
/// broken to the width of the part of its own piece this sheet carries, so a
/// reader can tell whose the words are. It gives way to the node names rather
/// than the other way round: a name is a coordinate a person matches between
/// two taped sheets, and the block is prose the legend repeats. And a block
/// that would have to cross another piece's cut line is not written here at
/// all, because paper that says a thing without saying whose is worse than
/// paper that says it one sheet less often.
///
/// Where it goes is then measured and not guessed: of every place the names
/// and the neighbours leave it, it takes the one printing the most of its ink
/// inside its own closed cut line, because ink outside that line is ink the
/// scissors throw away. On a tiled sheet the corner of the box around a
/// trouser leg can be a hand's width from any cloth at all, so the search runs
/// right and down across the cloth and not only under the names.
pub(super) fn laid(said: &[(f64, &str)], room: &Room<'_>) -> Option<Vec<Line>> {
    let budget = both(room.sheet, boxed(room.cloth))?;
    let lines = wrapped(said, budget[2] - budget[0]);
    let first = lines.first()?.0;
    let down = stacked(&lines);
    let widest = lines
        .iter()
        .map(|(size, body)| metric::wide(*size, body))
        .fold(0.0, f64::max);
    let (deep, last) = (*down.last()?, lines.last()?.0);
    let (top, floor) = (budget[1] + first, budget[3] - deep - BELOW * last);
    let mut best: Option<(f64, Vec<Line>)> = None;
    for left in lefts(budget, widest, room.sheet) {
        for base in roosts(room.taken, [left, left + widest], top, floor, first) {
            if base + deep + BELOW * last > room.sheet[3] {
                break;
            }
            let held = placed(&lines, left, base, &down);
            if !clear(&held, room.taken) || held.iter().any(|line| enters(line, room.others)) {
                continue;
            }
            let home = home(&held, room.cloth);
            if home >= HOME {
                return Some(held);
            }
            if best.as_ref().is_none_or(|(found, _)| home > *found) {
                best = Some((home, held));
            }
        }
    }
    best.map(|(_, held)| held)
}

/// Every left edge the block may open at, the one it would rather first.
///
/// Its own corner, and then steps right across the part of its own box this
/// sheet carries. Running right is what puts a block on the cloth when the
/// cloth of this sheet is a tongue down one side of the cell: there is nothing
/// below the names to drop to that is any nearer it.
///
/// The last step is as far right as the block still fits inside its own box,
/// so no candidate pushes a line past the piece a reader is meant to read it
/// against. A block with no room to move answers with the one place it has,
/// and with nowhere at all when even that would run off the paper.
fn lefts(budget: [f64; 4], widest: f64, sheet: [f64; 4]) -> Vec<f64> {
    let home = budget[0] + (sheet[2] - budget[0] - widest).clamp(0.0, INSET);
    let room = budget[2] - widest - home;
    if room <= 0.0 {
        return Vec::from_iter((home + widest <= sheet[2]).then_some(home));
    }
    (0..=STEPS)
        .map(|step| home + room * f64::from(step) / f64::from(STEPS))
        .collect()
}

/// The box around the piece's cut line on this sheet's own plane.
fn boxed(cloth: &[[f64; 2]]) -> [f64; 4] {
    let (low, high) = units::box_of(cloth.iter().copied());
    [low[0], low[1], high[0], high[1]]
}

/// What share of a block's ink would be printed inside its own cut line.
///
/// Weighted by the room each line takes and not counted line by line, so a
/// title that landed on the cloth does not outvote four captions that did not.
///
/// A low share is not a reason to refuse. A piece too small to stand its own
/// words on says them beside itself rather than not at all: the narrowest
/// piece of the owner's jeans is five centimetres against a block seven wide,
/// and a sheet carrying a nameless outline is the worse of the two answers.
fn home(lines: &[Line], cloth: &[[f64; 2]]) -> f64 {
    let mut ink = 0.0;
    let mut held = 0.0;
    for line in lines {
        let box_of = metric::box_of(line.size, line.at, &line.body);
        let cell = [box_of[2] - box_of[0], box_of[3] - box_of[1]];
        let room = cell[0] * cell[1];
        ink += room;
        held += room * lands::share(cloth, [box_of[0], box_of[1]], cell);
    }
    if ink > 0.0 { held / ink } else { 1.0 }
}

/// Whether every line of the block is clear of everything already written.
///
/// Line by line and not by the box around the whole block: a name beside the
/// one short line of a block that is otherwise wide touches that box and
/// touches no word of it, and refusing the place for it would send the block
/// down the sheet for nothing.
fn clear(lines: &[Line], taken: &[[f64; 4]]) -> bool {
    !lines.iter().any(|line| {
        let held = metric::box_of(line.size, line.at, &line.body);
        taken.iter().any(|taken| meets(held, *taken))
    })
}

/// The part of the sheet the piece's own box reaches, where it reaches any.
fn both(sheet: [f64; 4], piece: [f64; 4]) -> Option<[f64; 4]> {
    let held = [
        sheet[0].max(piece[0]),
        sheet[1].max(piece[1]),
        sheet[2].min(piece[2]),
        sheet[3].min(piece[3]),
    ];
    (held[0] < held[2] && held[1] < held[3]).then_some(held)
}

/// Every baseline the block could open at: the top of its own box, just under
/// each line already written across the place it wants, and then down its own
/// box a step at a time.
///
/// Only what stands in the way earns a gap of its own. A name to the left or
/// the right of the block costs it nothing, and a block that dropped under
/// every name on the sheet would end up nowhere near the piece it names.
///
/// The steps are there because nothing else reaches the cloth on a sheet that
/// carries a band of its piece across the middle: the gap under a name is
/// still at the top of the paper and the cloth is not.
///
/// The first baseline is offered whether or not the block fits under it, so on
/// a sheet whose slice of the piece is shallower than the block the words do
/// hang past it — 19.38 mm at the worst. That is the lesser harm and the one
/// `laid` already chose: a sentence beside its cloth still reads, and a piece
/// with no block says nothing. Only the sheet itself stops it.
fn roosts(taken: &[[f64; 4]], across: [f64; 2], from: f64, to: f64, size: f64) -> Vec<f64> {
    let mut out = vec![from];
    for held in taken {
        if held[0] < across[1] && held[2] > across[0] {
            out.push(held[3] + ABOVE * size);
        }
    }
    if to > from {
        let room = to - from;
        out.extend((1..=STEPS).map(|step| from + room * f64::from(step) / f64::from(STEPS)));
    }
    out.retain(|&base| base >= from);
    out.sort_by(f64::total_cmp);
    out
}

/// Each line of the block at its own baseline, from the first.
fn placed(lines: &[(f64, String)], left: f64, base: f64, down: &[f64]) -> Vec<Line> {
    lines
        .iter()
        .zip(down)
        .map(|((size, body), down)| Line {
            size: *size,
            body: body.clone(),
            at: [left, base + down],
        })
        .collect()
}

/// Whether two boxes share any paper at all.
fn meets(one: [f64; 4], two: [f64; 4]) -> bool {
    one[0] < two[2] && two[0] < one[2] && one[1] < two[3] && two[1] < one[3]
}

/// Whether a line of words reaches any cloth but its own.
fn enters(line: &Line, others: &[&[[f64; 2]]]) -> bool {
    let held = metric::box_of(line.size, line.at, &line.body);
    let size = [held[2] - held[0], held[3] - held[1]];
    others
        .iter()
        .any(|outline| lands::on(outline, [held[0], held[1]], size))
}

#[cfg(test)]
mod tests;
