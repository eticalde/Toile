use serde::{Deserialize, Serialize};

use crate::{DocError, EdgeRange, PieceKey};

/// An axis a piece is repeated across.
///
/// The axis is a stretch of the piece's own contour and not two loose points.
/// A fold is an edge of the drawing — the cloth turns back on it — so naming
/// it the way a seam side and an elastic are named keeps it there: the ends
/// are node keys with a fraction local to the tract leaving them, and a node
/// inserted inside the stretch moves neither of them. Two loose points could
/// name a line the contour does not run along, and a fold that is not an edge
/// is not a fold.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Symmetry {
    /// The stretch of contour the axis runs along, head to tail the way the
    /// contour runs.
    pub axis: EdgeRange,
    /// What the repetition produces.
    pub kind: SymmetryKind,
}

/// What a symmetry produces on the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SymmetryKind {
    /// One continuous piece, folded on the axis: no seam down the middle.
    Fold,
    /// Two pieces, each cut on its own.
    Mirror,
}

impl Symmetry {
    /// A piece drawn half against `axis`, with the other half its mirror.
    pub fn fold(axis: EdgeRange) -> Symmetry {
        Symmetry {
            axis,
            kind: SymmetryKind::Fold,
        }
    }

    /// The piece the axis belongs to, when both its ends agree on one.
    pub fn piece(self) -> Option<PieceKey> {
        self.axis.piece()
    }

    /// Refuses an axis no piece can be repeated across.
    ///
    /// The ends have to sit on one piece, and they have to be two places: an
    /// axis whose ends are one point is a direction nobody named, and every
    /// reflection across it is undefined.
    ///
    /// A mirror is refused here and not only at the command, because the file
    /// is the other way in. Nothing writes one yet, and a reader that took one
    /// and drew a single piece would be cutting half a garment with nothing on
    /// screen to say so.
    pub(crate) fn check(self) -> Result<(), DocError> {
        if self.axis.piece().is_none() {
            return Err(DocError::SplitSymmetry);
        }
        if self.axis.head == self.axis.tail {
            return Err(DocError::FoldAxis);
        }
        if !self.axis.head.is_valid() || !self.axis.tail.is_valid() {
            return Err(DocError::AnchorFraction);
        }
        match self.kind {
            SymmetryKind::Fold => Ok(()),
            SymmetryKind::Mirror => Err(DocError::NotYetImplemented),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EdgeAnchor, PointKey};

    fn axis() -> EdgeRange {
        EdgeRange::between(
            PieceKey::new(0, 0),
            PointKey::new(1, 0),
            PointKey::new(2, 0),
        )
    }

    #[test]
    fn a_fold_and_a_mirror_are_not_the_same_axis() {
        let fold = Symmetry::fold(axis());
        assert_eq!(fold.kind, SymmetryKind::Fold);
        assert_eq!(fold.piece(), Some(PieceKey::new(0, 0)));
        assert_ne!(
            fold,
            Symmetry {
                axis: axis(),
                kind: SymmetryKind::Mirror
            }
        );
    }

    #[test]
    fn an_axis_across_two_pieces_or_onto_one_place_is_no_axis() {
        assert_eq!(Symmetry::fold(axis()).check(), Ok(()));
        let split = EdgeRange {
            tail: EdgeAnchor::at_node(PieceKey::new(1, 0), PointKey::new(2, 0)),
            ..axis()
        };
        assert_eq!(
            Symmetry::fold(split).check(),
            Err(DocError::SplitSymmetry),
            "an axis has to lie on one piece"
        );
        let pinched = EdgeRange {
            tail: axis().head,
            ..axis()
        };
        assert_eq!(Symmetry::fold(pinched).check(), Err(DocError::FoldAxis));
    }

    /// Two pieces cut from one drawing are not one bigger piece, so nothing
    /// downstream of the unfold answers for them: the mirror waits for its own
    /// tool and is refused meanwhile, at the command and in the file alike.
    #[test]
    fn a_mirror_is_refused_until_something_can_cut_the_second_piece() {
        let mirror = Symmetry {
            axis: axis(),
            kind: SymmetryKind::Mirror,
        };
        assert_eq!(mirror.check(), Err(DocError::NotYetImplemented));
    }
}
