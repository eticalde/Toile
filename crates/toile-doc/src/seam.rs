use crate::EdgeRange;

/// Two stretches of contour sewn to each other.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Seam {
    /// One side of the seam.
    pub a: EdgeRange,
    /// The other side.
    pub b: EdgeRange,
    /// Whether the two sides run the same way round their contours.
    pub orientation: SeamOrientation,
    /// What the seam does with a difference in length.
    pub kind: SeamKind,
    /// How much difference it takes before the seam complains; `None` reads
    /// the document variable instead.
    pub tolerance: Option<f64>,
}

/// Which way the second stretch runs against the first.
///
/// Carrying the direction on the seam is what removes the old convention of
/// passing a range backwards to mean the same thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeamOrientation {
    /// Both stretches run the same way.
    Aligned,
    /// The second runs against the first.
    Opposed,
}

/// What a seam expects of the two lengths it joins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SeamKind {
    /// The two sides are meant to measure the same.
    Plain,
    /// One side is meant to be longer, and eased in.
    Eased {
        /// The intended excess, in centimetres.
        expected_cm: f64,
    },
    /// One side is gathered onto the other at a ratio.
    Gathered {
        /// Centimetres of the long side per centimetre of the short one.
        ratio: f64,
    },
}

impl Seam {
    /// Name of the document variable a seam falls back to for its tolerance.
    pub const TOLERANCE_VARIABLE: &'static str = "tolerancia_costura";
    /// Name of the variable a gathered seam falls back to for its ratio.
    pub const RATIO_TOLERANCE_VARIABLE: &'static str = "tolerancia_ratio";
    /// Centimetres of length mismatch a new document lets a seam carry.
    pub const DEFAULT_TOLERANCE_CM: f64 = 0.5;
    /// Ratio mismatch a new document lets a gathered seam carry.
    pub const DEFAULT_RATIO_TOLERANCE: f64 = 0.05;

    /// A seam that joins two stretches with no ease and no gathering.
    pub fn plain(a: EdgeRange, b: EdgeRange, orientation: SeamOrientation) -> Seam {
        Seam {
            a,
            b,
            orientation,
            kind: SeamKind::Plain,
            tolerance: None,
        }
    }

    /// The centimetres of difference the seam is meant to carry.
    pub fn expected_cm(&self) -> f64 {
        match self.kind {
            SeamKind::Plain | SeamKind::Gathered { .. } => 0.0,
            SeamKind::Eased { expected_cm } => expected_cm,
        }
    }

    /// The name of the document variable this seam's tolerance falls back to.
    pub fn tolerance_variable(&self) -> &'static str {
        match self.kind {
            SeamKind::Plain | SeamKind::Eased { .. } => Seam::TOLERANCE_VARIABLE,
            SeamKind::Gathered { .. } => Seam::RATIO_TOLERANCE_VARIABLE,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "a seam stores the centimetres it was given"
    )]

    use super::*;
    use crate::{PieceKey, PointKey};

    fn range() -> EdgeRange {
        EdgeRange::between(
            PieceKey::new(0, 0),
            PointKey::new(0, 0),
            PointKey::new(1, 0),
        )
    }

    #[test]
    fn a_plain_seam_expects_no_difference() {
        let seam = Seam::plain(range(), range(), SeamOrientation::Opposed);
        assert_eq!(seam.expected_cm(), 0.0);
        assert_eq!(seam.tolerance, None);
        assert_eq!(seam.tolerance_variable(), "tolerancia_costura");
    }

    #[test]
    fn an_eased_seam_expects_the_centimetres_it_declares() {
        let seam = Seam {
            kind: SeamKind::Eased { expected_cm: 1.0 },
            ..Seam::plain(range(), range(), SeamOrientation::Aligned)
        };
        assert_eq!(seam.expected_cm(), 1.0);
        assert_eq!(seam.tolerance_variable(), "tolerancia_costura");
    }

    #[test]
    fn a_gathered_seam_is_judged_on_its_ratio() {
        let seam = Seam {
            kind: SeamKind::Gathered { ratio: 2.0 },
            ..Seam::plain(range(), range(), SeamOrientation::Aligned)
        };
        assert_eq!(seam.expected_cm(), 0.0);
        assert_eq!(seam.tolerance_variable(), "tolerancia_ratio");
    }
}
