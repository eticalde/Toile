use super::super::units::{box_of, millimetres};
use super::grid::Grid;
use super::paper::Paper;
use crate::draft::PieceKey;

/// Blank paper between two pieces that share a sheet, in millimetres.
///
/// Two things have to fit in it. A hand following a printed line with scissors
/// wanders about a millimetre, so two neighbours cut by that hand need twice
/// that plus the weight of both lines before one cut reaches the other's line —
/// under three. The wider of the two is the node names: a name is set half a
/// caption above the node it belongs to, so one on a piece's top edge rises a
/// caption and a half into the paper above it, and it has to stay in the half
/// of the gap nearer its own piece.
///
/// Ten is what this sheet already calls clear paper — the blank band it leaves
/// between the pattern and the square that proves the scale — so the page has
/// one idea of clear and not two. What it does not have to be wider than is a
/// seam allowance: this pattern draws none, so no line on the paper is ever
/// anything but a line to cut on.
pub(super) const GAP: f64 = 10.0;

/// One piece placed on a pile's plane.
#[derive(Clone)]
pub(super) struct Placed {
    /// Which piece of the document it is.
    pub(super) piece: PieceKey,
    /// Its outline, in the centimetres the document counts in.
    pub(super) outline: Vec<[f64; 2]>,
    /// Where the box around it opens, in centimetres.
    pub(super) low: [f64; 2],
    /// What it measures, in millimetres.
    pub(super) size: [f64; 2],
    /// How many sheets it would take with a plane to itself.
    pub(super) alone: usize,
    /// Where its box opens on the plane, in millimetres.
    pub(super) place: [f64; 2],
}

impl Placed {
    /// A piece at the corner of a plane of its own, which is where every piece
    /// starts and where one asked for on its own stays.
    pub(super) fn new(piece: PieceKey, outline: Vec<[f64; 2]>) -> Placed {
        let (low, high) = box_of(outline.iter().copied());
        Placed {
            piece,
            outline,
            low,
            size: millimetres([high[0] - low[0], high[1] - low[1]]),
            alone: 0,
            place: [0.0, 0.0],
        }
    }

    /// Where the far corner of its box falls on the plane, in millimetres.
    fn far(&self) -> [f64; 2] {
        [self.place[0] + self.size[0], self.place[1] + self.size[1]]
    }
}

/// The piles a product's pieces are laid on, in the order the file prints them.
///
/// A piece joins a pile it costs nothing: the pile takes no more sheets with it
/// than without, so no piece ever loses paper to a neighbour and the small ones
/// ride along on paper that was blank. It joins only a pile every sheet of
/// which then carries a part of every piece on it, which is what keeps one pile
/// one thing — a stack of sheets that are all the same pieces, instead of a
/// stack in which three sheets out of eighteen quietly carry a waistband as
/// well.
///
/// The pieces are walked in the document's own order and a pile opens where its
/// first piece would have been, so the order the file prints is still the order
/// the pattern draws.
pub(super) fn piles(paper: Paper, ready: Vec<Placed>) -> Vec<Vec<Placed>> {
    let mut piles: Vec<Vec<Placed>> = Vec::new();
    for mut piece in ready {
        let room = piles
            .iter()
            .enumerate()
            .find_map(|(rank, pile)| fits(paper, pile, &piece).map(|place| (rank, place)));
        match room {
            Some((rank, place)) => {
                piece.place = place;
                piles[rank].push(piece);
            }
            None => piles.push(vec![piece]),
        }
    }
    piles
}

/// Where a piece may go on a pile's plane, when it may go there at all.
///
/// Two places are tried and the first that costs nothing wins: beside what is
/// already on the plane's lowest shelf, and below everything on it. Beside
/// comes first because a shelf is filled before another is opened — that is
/// what puts two small pieces on one sheet rather than one under the other,
/// which on paper taller than it is wide would be the shape that asks for a
/// second sheet.
fn fits(paper: Paper, pile: &[Placed], piece: &Placed) -> Option<[f64; 2]> {
    let sheets = Grid::new(paper, pile).ok()?.sheets();
    [beside(pile), below(pile)]
        .into_iter()
        .find(|&place| free(paper, pile, piece, place, sheets))
}

/// Whether a piece placed there costs the pile nothing and lands on all of it.
fn free(paper: Paper, pile: &[Placed], piece: &Placed, place: [f64; 2], sheets: usize) -> bool {
    let mut trial = pile.to_vec();
    let mut joining = piece.clone();
    joining.place = place;
    trial.push(joining);
    Grid::new(paper, &trial).is_ok_and(|grid| grid.sheets() <= sheets && grid.one_pile())
}

/// Beside what is on the plane's lowest shelf: a gap to the right of the piece
/// on it that reaches furthest across.
fn beside(pile: &[Placed]) -> [f64; 2] {
    let shelf = pile
        .iter()
        .map(|laid| laid.place[1])
        .fold(0.0_f64, f64::max);
    let across = pile
        .iter()
        .filter(|laid| laid.place[1] >= shelf)
        .map(|laid| laid.far()[0])
        .fold(0.0_f64, f64::max);
    [across + GAP, shelf]
}

/// Below everything on the plane, against its left edge.
fn below(pile: &[Placed]) -> [f64; 2] {
    let down = pile
        .iter()
        .map(|laid| laid.far()[1])
        .fold(0.0_f64, f64::max);
    [0.0, down + GAP]
}

#[cfg(test)]
mod tests;
