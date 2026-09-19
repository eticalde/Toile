use serde::{Deserialize, Serialize};

use crate::{DocError, EdgeRange};

/// A stretch of one piece's contour held to a ratio of the length it was drawn
/// at, and how hard it is held there.
///
/// This is what keeps a garment on a body. A waistband at 85 % pulls the cloth
/// in until the body stops it, and what is left in it is tension: an elastic of
/// finite strength that cannot close around a body lets the garment slide, and
/// that is the truth about the fit rather than a failure to report it.
///
/// It sits on a stretch of contour because that is the shape of the thing being
/// described: a waistband is a length of one edge, not a join between two
/// pieces and not the whole cloth.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Elastic {
    /// The stretch of contour it holds.
    ///
    /// An `EdgeRange` for the reason a seam's side is one: its ends are node
    /// keys with a fraction local to the tract leaving them, so a node inserted
    /// inside the stretch moves neither end.
    pub at: EdgeRange,
    /// The length it pulls that stretch to, as a fraction of the drawn length.
    pub ratio: f64,
    /// How hard it pulls: a stiffness, never a distance.
    pub strength: f64,
}

impl Elastic {
    /// The ratio of a stretch held to the very length it was drawn at.
    pub const NEUTRAL_RATIO: f64 = 1.0;

    /// The longest an elastic is allowed to stand in for: twice the drawn
    /// length.
    pub const MAX_RATIO: f64 = 2.0;

    /// The slackest an elastic is written at: a hundredth of a band of
    /// strength one.
    ///
    /// A floor, and not only "above zero", because of what a strength becomes.
    /// The stretch an elastic holds runs at the elastic's stiffness in place of
    /// the cloth's own, so a number small enough asks for no band and gets no
    /// cloth either: a hand-typed `1e-300` is positive, finite, and a waistline
    /// nothing holds to any length. That is a different garment from the one
    /// with no elastic on it, and no elastic is what such a number meant.
    pub const MIN_STRENGTH: f64 = 0.01;

    /// An elastic holding `at` to `ratio` of its drawn length, at `strength`.
    pub fn new(at: EdgeRange, ratio: f64, strength: f64) -> Elastic {
        Elastic {
            at,
            ratio,
            strength,
        }
    }

    /// Refuses a ratio or a strength no elastic carries.
    ///
    /// The same door keeps out the numbers JSON cannot spell: a NaN or an
    /// infinity fails every comparison here, and the writer would spell one
    /// `null` and refuse the whole product on the next open. A ratio of zero is
    /// refused with them — a stretch pulled to no length at all is not an
    /// elastic — and so is a strength under [`Elastic::MIN_STRENGTH`], zero
    /// included, which holds nothing and takes the cloth's own hold with it.
    pub(crate) fn check(self) -> Result<(), DocError> {
        let holds = self.ratio > 0.0 && self.ratio <= Elastic::MAX_RATIO;
        if !holds {
            return Err(DocError::ElasticRatio);
        }
        let pulls = self.strength >= Elastic::MIN_STRENGTH && self.strength.is_finite();
        if !pulls {
            return Err(DocError::ElasticStrength);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, reason = "an elastic stores the ratio it was given")]

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
    fn an_elastic_at_the_neutral_ratio_asks_for_the_drawn_length() {
        let elastic = Elastic::new(range(), Elastic::NEUTRAL_RATIO, 30.0);
        assert_eq!(elastic.check(), Ok(()));
        assert_eq!(elastic.ratio, 1.0);
        assert_eq!(elastic.at, range());
    }

    #[test]
    fn a_ratio_outside_what_an_elastic_holds_is_refused() {
        for ratio in [0.0, -0.5, 2.5, f64::NAN, f64::INFINITY] {
            assert_eq!(
                Elastic::new(range(), ratio, 30.0).check(),
                Err(DocError::ElasticRatio),
                "{ratio}"
            );
        }
        for ratio in [f64::MIN_POSITIVE, 0.85, Elastic::MAX_RATIO] {
            assert_eq!(
                Elastic::new(range(), ratio, 30.0).check(),
                Ok(()),
                "{ratio}"
            );
        }
    }

    #[test]
    fn a_strength_that_is_not_finite_and_positive_is_refused() {
        for strength in [0.0, -30.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                Elastic::new(range(), 0.85, strength).check(),
                Err(DocError::ElasticStrength),
                "{strength}"
            );
        }
    }

    /// Positive and finite is not enough: the floor is a number, the floor
    /// itself passes, and everything the strength rail offers stands above it.
    #[test]
    fn a_strength_under_the_floor_is_refused_and_the_floor_is_not() {
        let under = Elastic::MIN_STRENGTH / 2.0;
        for strength in [1.0e-300, f64::MIN_POSITIVE, 1.0e-6, under] {
            assert_eq!(
                Elastic::new(range(), 0.85, strength).check(),
                Err(DocError::ElasticStrength),
                "{strength}"
            );
        }
        for strength in [Elastic::MIN_STRENGTH, 0.1, 10.0, 50.0, 1.0e300] {
            assert_eq!(
                Elastic::new(range(), 0.85, strength).check(),
                Ok(()),
                "{strength}"
            );
        }
    }
}
