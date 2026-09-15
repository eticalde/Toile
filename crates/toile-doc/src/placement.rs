use serde::{Deserialize, Serialize};

use crate::DocError;

/// Where a piece sits on the product overview: a translation, in centimetres.
///
/// Layout, never geometry. The contour keeps the coordinates its formulas give
/// it and the overview draws it this far from there, so arranging a piece
/// rewrites no formula its author wrote. Nothing that resolves, flattens,
/// meshes, sews or drapes a piece reads it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    /// Centimetres across the page.
    pub x: f64,
    /// Centimetres down the page, the way the document's y grows.
    pub y: f64,
}

impl Placement {
    /// The piece drawn `x` centimetres across and `y` down from where its
    /// contour puts it.
    pub const fn new(x: f64, y: f64) -> Placement {
        Placement { x, y }
    }

    /// Refuses a translation JSON cannot spell.
    ///
    /// The writer would spell it `null`, and the next open would refuse the
    /// whole product.
    pub(crate) fn check(self) -> Result<(), DocError> {
        if self.x.is_finite() && self.y.is_finite() {
            Ok(())
        } else {
            Err(DocError::NonFinite("placement".to_owned()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_placement_with_a_coordinate_that_is_not_finite_is_refused() {
        assert_eq!(Placement::new(-12.5, 0.0).check(), Ok(()));
        for (x, y) in [
            (f64::NAN, 0.0),
            (0.0, f64::INFINITY),
            (f64::NEG_INFINITY, f64::NAN),
        ] {
            assert_eq!(
                Placement::new(x, y).check(),
                Err(DocError::NonFinite("placement".to_owned()))
            );
        }
    }
}
