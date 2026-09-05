use serde::{Deserialize, Serialize};

use crate::PointKey;

/// An axis a piece is repeated across.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Symmetry {
    /// The two points the axis runs through.
    pub axis: (PointKey, PointKey),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fold_and_a_mirror_are_not_the_same_axis() {
        let axis = (PointKey::new(0, 0), PointKey::new(8, 0));
        let fold = Symmetry {
            axis,
            kind: SymmetryKind::Fold,
        };
        assert_ne!(
            fold,
            Symmetry {
                axis,
                kind: SymmetryKind::Mirror
            }
        );
    }
}
