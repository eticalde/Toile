use toile_engine::draft::{Draft, EdgeRange, PieceKey, PointKey, SeamOrientation};

use super::super::layout::Laid;
use super::super::pick::away;
use super::super::tract::{self, Tract};

/// Said when a tract asked onto a side does not touch either end of it.
pub const APART: &str = "ese tramo no es contiguo al lado: un lado es un solo trecho del contorno";
/// Said when a tract is asked out of the middle of a side.
pub const MIDDLE: &str = "un tramo solo sale de un lado por uno de sus extremos";
/// Said when one more tract would take a side right round its piece.
pub const LOOP: &str = "un lado no puede dar la vuelta entera a la pieza";
/// Said when the last tract of a sewn side is asked out of it.
pub const LAST: &str = "a un lado cosido le queda al menos un tramo";

/// One piece's tracts where the whole product draws them, in centimetres of
/// the mat and not of the piece.
#[derive(Debug, Clone, PartialEq)]
pub struct Spread {
    /// The piece.
    pub piece: PieceKey,
    /// Its tracts in contour order, moved to where the overview laid it.
    pub tracts: Vec<Tract>,
}

/// One side of a seam as it is picked: a run of contiguous tracts of one
/// piece, which is one tract when nobody has lengthened it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pick {
    /// The piece the run belongs to.
    pub piece: PieceKey,
    /// The node its first tract leaves.
    pub from: PointKey,
    /// The node its last tract runs to.
    pub to: PointKey,
}

impl Pick {
    /// The stretch of contour the run names, node to node.
    pub fn range(self) -> EdgeRange {
        EdgeRange::between(self.piece, self.from, self.to)
    }

    /// The run a stretch names, when it lies node to node on one piece.
    ///
    /// A stretch anchored part way along a tract is not a run of whole tracts,
    /// so there is no end of it a whole tract could be added to.
    pub fn of(range: EdgeRange) -> Option<Pick> {
        let on_nodes = range.head.t.to_bits() == 0.0_f64.to_bits()
            && range.tail.t.to_bits() == 0.0_f64.to_bits();
        let piece = range.piece().filter(|_| on_nodes)?;
        Some(Pick {
            piece,
            from: range.head.from,
            to: range.tail.from,
        })
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

/// The line a run draws on the mat now; nothing once its piece or one of its
/// ends has gone.
pub fn line_of(spread: &[Spread], run: Pick) -> Option<Vec<[f64; 2]>> {
    let on = spread.iter().find(|it| it.piece == run.piece)?;
    let line = stretch(&on.tracts, run.range());
    (line.len() >= 2).then_some(line)
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

/// Which way round two sides are sewn, from how they lie on the mat.
///
/// Pieces laid side by side the same way up are sewn top to top and bottom to
/// bottom, whichever way each contour happens to run past the seam. So the
/// ends that lie nearest each other are the ends that meet: when pairing head
/// with head is the shorter reach the sides run aligned, and otherwise they
/// run opposed. Only the ends are read, so a run of several tracts answers as
/// one tract does. It is a first answer, and the inspector flips it.
pub fn facing(a: &[[f64; 2]], b: &[[f64; 2]]) -> SeamOrientation {
    let ends = |line: &[[f64; 2]]| Some((*line.first()?, *line.last()?));
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

/// Where a tract stands against a run of its own piece: how many tracts past
/// the head of the run it lies, how many tracts the run has, and how many the
/// contour has. Nothing when the two are on different pieces, or when either
/// names a node the contour no longer runs through.
fn place(spread: &[Spread], run: Pick, tract: Pick) -> Option<[usize; 3]> {
    let on = spread.iter().find(|it| it.piece == run.piece)?;
    let tracts = (tract.piece == run.piece).then_some(&on.tracts)?;
    let count = tracts.len();
    let at = |node: PointKey| tracts.iter().position(|it| it.node == node);
    let (start, asked) = (at(run.from)?, at(tract.from)?);
    let last = (0..count).position(|k| tracts[(start + k) % count].to == run.to)?;
    Some([(asked + count - start) % count, last + 1, count])
}

/// Whether a tract is one of the tracts a run is made of.
pub fn holds(spread: &[Spread], run: Pick, tract: Pick) -> bool {
    place(spread, run, tract).is_some_and(|[offset, length, _]| offset < length)
}

/// What a shift-press on one tract makes of a run.
///
/// Shift adds the tract to the run and takes it back out when it was already
/// there, as it does with nodes. What a run adds to that is that it has to
/// stay one stretch of contour, because one `EdgeRange` is what it is stored
/// as: a tract comes in only at either end and goes out only from either end,
/// and anything else is refused in words. `None` is a run whose only tract
/// went.
pub fn shifted(spread: &[Spread], run: Pick, tract: Pick) -> Result<Option<Pick>, &'static str> {
    let Some([offset, length, count]) = place(spread, run, tract) else {
        return Err(APART);
    };
    // A run of every tract would end on the node it starts from, and a stretch
    // whose two ends are one place is a stretch of no length at all.
    let grown = |run: Pick| {
        if length + 1 >= count {
            Err(LOOP)
        } else {
            Ok(Some(run))
        }
    };
    let (from, to) = (tract.from, tract.to);
    match offset {
        0 if length == 1 => Ok(None),
        0 => Ok(Some(Pick { from: to, ..run })),
        _ if offset + 1 == length => Ok(Some(Pick { to: from, ..run })),
        _ if offset < length => Err(MIDDLE),
        _ if offset == length => grown(Pick { to, ..run }),
        _ if offset + 1 == count => grown(Pick { from, ..run }),
        _ => Err(APART),
    }
}

/// The line a stretch of contour draws, from its head anchor to its tail.
///
/// The anchors' fractions are honoured, so a seam that starts part way along a
/// tract is drawn from there and not from the node behind it. A tail behind
/// its head on the same tract goes the long way round, as the contour runs. A
/// stretch whose two ends are one place draws nothing: the engine pairs
/// nothing along it, and a thread round the whole piece would say it did.
pub fn stretch(tracts: &[Tract], at: EdgeRange) -> Vec<[f64; 2]> {
    let Some(start) = tracts.iter().position(|it| it.node == at.head.from) else {
        return Vec::new();
    };
    if at.head.from == at.tail.from {
        if at.tail.t.to_bits() == at.head.t.to_bits() {
            return Vec::new();
        }
        if at.tail.t > at.head.t {
            return cut(&tracts[start].line, at.head.t, at.tail.t);
        }
    }
    let mut out = cut(&tracts[start].line, at.head.t, 1.0);
    for step in 1..=tracts.len() {
        let tract = &tracts[(start + step) % tracts.len()];
        if tract.node == at.tail.from {
            if at.tail.t > 0.0 {
                out.extend(cut(&tract.line, 0.0, at.tail.t));
            }
            return out;
        }
        out.extend(tract.line.iter().copied());
    }
    Vec::new()
}

/// The part of a line between two fractions of its own length.
fn cut(line: &[[f64; 2]], from: f64, to: f64) -> Vec<[f64; 2]> {
    let total: f64 = line.windows(2).map(|pair| away(pair[0], pair[1])).sum();
    if total <= f64::EPSILON || (from <= 0.0 && to >= 1.0) {
        return line.to_vec();
    }
    let (lo, hi) = (from.clamp(0.0, 1.0) * total, to.clamp(0.0, 1.0) * total);
    let mut out = Vec::new();
    let mut along = 0.0;
    for pair in line.windows(2) {
        let span = away(pair[0], pair[1]);
        let at = |arc: f64| {
            let t = if span > 0.0 {
                (arc - along) / span
            } else {
                0.0
            };
            [0, 1].map(|k| pair[0][k] + (pair[1][k] - pair[0][k]) * t)
        };
        if lo >= along && lo <= along + span {
            out.push(at(lo));
        }
        if along + span > lo && along + span < hi {
            out.push(pair[1]);
        }
        if hi >= along && hi <= along + span {
            out.push(at(hi));
            break;
        }
        along += span;
    }
    out
}

#[cfg(test)]
mod tests;
