use super::super::sew::Sewn;
use crate::couture::Pin;

#[cfg(test)]
mod tests;

/// How far past a sewn stretch's own ends a declaration may be carried, as a
/// multiple of that stretch's abscissa run.
///
/// Its own length again, which is generous where it has to be and strict where
/// it matters. The line a declaration names is ordinarily an end of the very
/// stretch that carries it — a collar sewn across three chest lines meets the
/// centre front at its own last node — and the mesher leaves the two readings
/// a boundary sample apart, 4.5 mm of cloth measured on this bench. What it
/// refuses is the other order of magnitude: a pin sixteen stretch-lengths out.
const REACH: f64 = 1.0;

/// The heading one piece is turned by, and how it came to it.
#[derive(Debug, Clone, Copy)]
pub(super) struct Turned {
    /// How many seams away the declaration was written.
    pub(super) seams: usize,
    /// Where the declaration stood among the document's own, highest first.
    pub(super) rank: usize,
    /// The line of cloth this piece is pinned at, in its own frame.
    pub(super) pin: Pin,
}

/// Which heading each piece of a product is turned by: the one written on it,
/// or the nearest one written, carried to it along the seams.
///
/// The same ladder [`super::group::rings`] climbs for the station, and for the
/// same reason. A garment is one object: a collar sewn to a chest line that
/// says it faces the front faces the front too, and a ring left to find its
/// own way round came out at exactly half a turn from the rest of the garment
/// — the middle of its first panel, which is where a surface with nothing
/// declared opens.
///
/// A declaration travels as a line of cloth and not as an angle, because the
/// angle alone says nothing about which of a piece's own abscissae is at it.
/// What carries it is the seam: see [`carried`].
pub(super) fn spread(sewn: &[Sewn], pieces: usize, pins: &[Pin]) -> Vec<Option<Turned>> {
    let mut found: Vec<Option<Turned>> = vec![None; pieces];
    for (rank, pin) in pins.iter().enumerate() {
        if found.get(pin.piece).is_some_and(Option::is_none) {
            found[pin.piece] = Some(Turned {
                seams: 0,
                rank,
                pin: *pin,
            });
        }
    }
    let mut front: Vec<usize> = (0..pieces).filter(|&p| found[p].is_some()).collect();
    let mut seams = 0;
    while !front.is_empty() {
        seams += 1;
        let mut next = step(sewn, &found, &front, pieces);
        // Written after the whole ring of equal distance is read, so two
        // declarations the same number of seams away are settled by which
        // hangs higher and never by which piece the loop reached first.
        next.sort_unstable_by_key(|&(piece, rank, _)| (piece, rank));
        front.clear();
        for (piece, rank, pin) in next {
            if found[piece].is_none() {
                found[piece] = Some(Turned { seams, rank, pin });
                front.push(piece);
            }
        }
    }
    found
}

/// Every piece one seam out from the front, with the heading carried onto it.
fn step(
    sewn: &[Sewn],
    found: &[Option<Turned>],
    front: &[usize],
    pieces: usize,
) -> Vec<(usize, usize, Pin)> {
    let mut next = Vec::new();
    for &piece in front {
        let Some(from) = found[piece] else { continue };
        for one in sewn {
            let Some(side) = one.sides.iter().position(|&p| p == piece) else {
                continue;
            };
            let other = one.sides[1 - side];
            if other >= pieces || found[other].is_some() {
                continue;
            }
            if let Some(pin) = carried(one, side, from.pin) {
                next.push((other, from.rank, pin));
            }
        }
    }
    next
}

/// The same declaration read on the piece across a seam, or `None` for a seam
/// that cannot carry one.
///
/// The two ends of a sewn stretch are the same two places of the garment on
/// either side of it, so they fix how an abscissa of one piece is read on the
/// other: at the stretch's own scale, and reversed where the two run against
/// each other. The sense is reversed with it, because a sense is written about
/// a piece's rising abscissa and the other piece's may rise the other way.
///
/// `None` when the stretch runs at one abscissa on either side — a side seam
/// straight down the pattern. Such a seam says where two pieces meet and
/// nothing about how far across either of them anything else is, and a pin
/// guessed from it would be a line of cloth nobody drew.
///
/// `None` too for a declaration standing further outside the stretch than
/// [`REACH`] allows, which is where the correspondence stops being one.
fn carried(one: &Sewn, side: usize, pin: Pin) -> Option<Pin> {
    let (from, to) = (one.ends[side], one.ends[1 - side]);
    let (run, across) = (from[1] - from[0], to[1] - to[0]);
    if run.abs() <= f64::EPSILON || across.abs() <= f64::EPSILON {
        return None;
    }
    let along = (pin.at - from[0]) / run;
    if along < -REACH || along > 1.0 + REACH {
        return None;
    }
    let scale = across / run;
    Some(Pin {
        piece: one.sides[1 - side],
        at: to[0] + (pin.at - from[0]) * scale,
        turn: pin.turn,
        leftward: pin.leftward * scale.signum(),
    })
}
