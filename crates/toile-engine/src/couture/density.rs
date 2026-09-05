use toile_geom::length;

/// Boundary spacing, in metres.
///
/// Read off the demo scene: its 2.31 m of perimeter carry the 256 samples the
/// goldens were tuned at, which is a sample every 9 mm.
const SPACING: f64 = 0.009;

/// Triangle area cap: a material resolution, not a function of the perimeter.
const MAX_AREA: f64 = 2.0e-5;

/// Fewest samples a piece is meshed at, however small it is.
const FEWEST: usize = 64;

/// Most samples a piece is meshed at, however large it is.
const MOST: usize = 512;

/// How finely a contour is sampled and meshed: samples, then triangle cap.
///
/// The contour it is handed is the flattened one, curves and all, so a bowed
/// tract is meshed for the cloth its arc needs and not for the cloth its chord
/// would have needed.
///
/// A trouser front is a metre longer than a bodice, and a boundary count tuned
/// for one starves the other. The spacing is what stays fixed; the count
/// follows the perimeter, bounded at both ends so a mistyped coordinate cannot
/// ask for a million triangles.
pub fn for_contour(contour: &[[f64; 2]]) -> (usize, f64) {
    let samples = (length::perimeter(contour) / SPACING).round();
    let samples = if samples.is_finite() {
        (samples as usize).clamp(FEWEST, MOST)
    } else {
        FEWEST
    };
    (samples, MAX_AREA)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "the triangle cap is handed back exactly as it is declared"
    )]

    use toile_geom::curve;

    use super::*;
    use crate::demo;

    #[test]
    fn the_bodice_lands_within_five_percent_of_its_own_constants() {
        let (samples, max_area) = for_contour(&demo::bodice_contour());
        let drift = (samples as f64 - 256.0).abs() / 256.0;
        assert!(drift < 0.05, "the bodice asked for {samples} samples");
        assert_eq!(max_area, 2.0e-5);
    }

    #[test]
    fn a_longer_contour_asks_for_more_samples() {
        let small = [[0.0, 0.0], [0.5, 0.0], [0.5, 0.5], [0.0, 0.5]];
        let large = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        assert!(for_contour(&small).0 < for_contour(&large).0);
        assert_eq!(for_contour(&large).0, 444);
    }

    #[test]
    fn a_bowed_tract_is_counted_along_its_arc_and_not_its_chord() {
        let square = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let mut bowed = vec![square[0]];
        bowed.extend(curve::flatten(
            square[1],
            [1.3, 0.33],
            [1.3, 0.67],
            square[2],
            24,
        ));
        bowed.extend([square[2], square[3]]);
        assert!(for_contour(&bowed).0 > for_contour(&square).0);
    }

    #[test]
    fn a_tiny_contour_still_gets_a_mesh() {
        let dot = [[0.0, 0.0], [0.001, 0.0], [0.001, 0.001]];
        assert_eq!(for_contour(&dot).0, FEWEST);
        assert_eq!(for_contour(&[]).0, FEWEST);
    }

    #[test]
    fn a_contour_that_does_not_measure_is_meshed_at_the_floor() {
        let broken = [[0.0, 0.0], [f64::NAN, 0.0], [1.0, 1.0]];
        assert_eq!(for_contour(&broken).0, FEWEST);
    }
}
