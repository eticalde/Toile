use toile_doc::{EdgeAnchor, PieceKey, PointKey};

use super::Draft;
use super::fold::Cloth;
use super::resolve::arc;

/// How short a length counts as none, in centimetres.
const EPS: f64 = 1.0e-7;

impl Draft {
    /// The cloth a piece is cut from, in centimetres with y downward: the whole
    /// outline, unfolded.
    ///
    /// The same line as [`Draft::flat_cm`] for a piece nobody folded, and the
    /// drawn half plus its mirror for one drawn against an axis. This is what
    /// the scissors follow and what the sheet of paper carries; `flat_cm` is
    /// what the hand edits.
    pub fn cloth_cm(&self, piece: PieceKey) -> &[[f64; 2]] {
        self.pieces
            .get(&piece)
            .map_or(&[], |held| held.good.cloth_cm())
    }

    /// The cloth a folded piece makes, in centimetres: its whole outline, the
    /// crease it is folded on, and the reflection that put it there.
    ///
    /// `None` for a piece drawn whole, which is the answer to "is this piece on
    /// the fold" as well.
    pub fn cloth(&self, piece: PieceKey) -> Option<&Cloth> {
        self.pieces.get(&piece)?.good.cloth.as_ref()
    }

    /// A piece's perimeter in centimetres, measured along the cloth.
    ///
    /// The cloth and not the drawn half, so that the number a panel shows is
    /// the piece and so that the fraction [`Draft::anchor_fraction`] reads
    /// multiplies back into a length on the same line the mesher built.
    pub fn perimeter_cm(&self, piece: PieceKey) -> f64 {
        self.pieces
            .get(&piece)
            .map_or(0.0, |held| held.good.perimeter())
    }

    /// Flattened arc length in centimetres up to each node of the drawn
    /// contour, and round to the first again.
    ///
    /// Rebuilt with the geometry on every edit and read fresh on every derive,
    /// which is what keeps a seam on its node instead of on a fraction of a
    /// perimeter that has since changed length.
    pub fn node_cum(&self, piece: PieceKey) -> &[f64] {
        self.pieces
            .get(&piece)
            .map_or(&[], |held| held.good.cum.as_slice())
    }

    /// Where an anchor sits right now, as a fraction of its piece's cloth.
    ///
    /// A reading, never a residence: the address a seam stores is the node, and
    /// this fraction is derived from the fresh node table on every call. The
    /// cloth is what it is a fraction of, because the boundary a seam or an
    /// elastic is read onto is the mesh's, and the mesh is built from the
    /// cloth.
    ///
    /// `None` when the piece is unknown, when the node is not on its contour,
    /// or when the place sits on the crease of a fold: a crease is the inside
    /// of the cloth, so there is no edge there to sew or to hold in.
    pub fn anchor_fraction(&self, at: &EdgeAnchor) -> Option<f64> {
        let held = self.pieces.get(&at.piece)?;
        let good = &held.good;
        let total = good.cum.last().copied()?;
        let along = arc(&good.points, &good.cum, at)?;
        let Some(cloth) = good.cloth.as_ref() else {
            return (total > 0.0).then(|| along / total);
        };
        let from_opening = (along - cloth.opens_at).rem_euclid(total);
        if from_opening > cloth.walk + EPS || cloth.perimeter <= 0.0 {
            return None;
        }
        Some(from_opening / cloth.perimeter)
    }

    /// The length in centimetres of the walk from one node to another, the way
    /// the drawn contour runs.
    ///
    /// The drawn contour and not the cloth: this is what a tract measures and
    /// what a panel puts beside an edge, and both ends of it are nodes somebody
    /// drew. Zero when either node is not on the piece, which is what a length
    /// nobody can point at is worth.
    pub fn run_length_cm(&self, piece: PieceKey, from: PointKey, to: PointKey) -> f64 {
        let Some(held) = self.pieces.get(&piece) else {
            return 0.0;
        };
        let at = |key: PointKey| held.good.points.iter().position(|&(p, _)| p == key);
        let (Some(from), Some(to)) = (at(from), at(to)) else {
            return 0.0;
        };
        let cum = &held.good.cum;
        if to >= from {
            cum[to] - cum[from]
        } else {
            cum[cum.len() - 1] - cum[from] + cum[to]
        }
    }
}
