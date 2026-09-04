/// `n` evenly spaced fractions across `[0, 1)`.
pub fn uniform_fractions(n: usize) -> Vec<f64> {
    (0..n).map(|i| i as f64 / n as f64).collect()
}

/// Samples a closed polygon at the given fractions of its perimeter.
///
/// Resampling an edited contour at the *same* fractions yields the same
/// number of boundary vertices in the same order, so mesh connectivity
/// survives a shape edit and the solver can warm-start from it.
///
/// `fractions` must be sorted ascending; the scan advances monotonically.
///
/// The arc-length table comes from `length::cumulative`, so a measurement a
/// person reads and the mesh they see are built on the same sum.
pub fn sample_closed(contour: &[[f64; 2]], fractions: &[f64]) -> Vec<[f64; 2]> {
    let n = contour.len();
    let cum = crate::length::cumulative(contour);
    let total = cum[n];

    let mut out = Vec::with_capacity(fractions.len());
    let mut seg = 0usize;
    for &f in fractions {
        let target = f * total;
        while seg + 1 < n && cum[seg + 1] < target {
            seg += 1;
        }
        let (a, b) = (contour[seg], contour[(seg + 1) % n]);
        let len = cum[seg + 1] - cum[seg];
        let t = if len > 0.0 {
            (target - cum[seg]) / len
        } else {
            0.0
        };
        out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unit square, corners only.
    fn square() -> Vec<[f64; 2]> {
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]
    }

    #[test]
    fn uniform_fractions_are_half_open() {
        assert_eq!(uniform_fractions(4), vec![0.0, 0.25, 0.5, 0.75]);
    }

    #[test]
    fn corner_fractions_land_on_corners() {
        let got = sample_closed(&square(), &uniform_fractions(4));
        assert_eq!(got, square());
    }

    #[test]
    fn midpoint_of_the_first_edge() {
        let got = sample_closed(&square(), &[0.125]);
        assert_eq!(got, vec![[0.5, 0.0]]);
    }

    #[test]
    fn sample_count_matches_fraction_count() {
        let f = uniform_fractions(37);
        assert_eq!(sample_closed(&square(), &f).len(), 37);
    }

    /// The five-node contour the bit table below was taken from.
    fn awkward() -> Vec<[f64; 2]> {
        vec![
            [0.0, 0.0],
            [0.37, 0.11],
            [0.53, 0.61],
            [0.19, 0.83],
            [-0.07, 0.42],
        ]
    }

    /// Every bit the sampler produced before the arc-length table was shared
    /// with `length::cumulative`.
    ///
    /// The drape golden rests on this function: a rounding difference here
    /// moves every rest length in the piece.
    #[test]
    fn sample_closed_is_unchanged_by_the_shared_cumulative() {
        let expected: [[u64; 2]; 7] = [
            [0x0000_0000_0000_0000, 0x0000_0000_0000_0000],
            [0x3fd3_84d9_48ec_a3c3, 0x3fb7_3632_d342_ec42],
            [0x3fdc_902c_d2c8_cb38, 0x3fd6_4cc9_8357_521a],
            [0x3fdf_9481_79eb_a290, 0x3fe4_46eb_3166_f714],
            [0x3fcc_f7a7_c022_85ae, 0x3fe9_cee7_1f69_d9cf],
            [0x3fa5_e50e_b284_eddc, 0x3fe3_2155_1126_c01c],
            [0xbfaa_c817_f8fa_2b38, 0x3fd4_1611_fabb_a069],
        ];
        let got = sample_closed(&awkward(), &uniform_fractions(7));
        let bits: Vec<[u64; 2]> = got
            .iter()
            .map(|p| [p[0].to_bits(), p[1].to_bits()])
            .collect();
        assert_eq!(bits, expected);
    }

    #[test]
    fn a_whole_turn_lands_back_on_the_first_node() {
        let got = sample_closed(&square(), &[1.0]);
        assert_eq!(got, vec![[0.0, 0.0]]);
    }

    #[test]
    fn a_degenerate_edge_does_not_divide_by_zero() {
        let dup = vec![[0.0, 0.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];
        assert!(
            sample_closed(&dup, &uniform_fractions(8))
                .iter()
                .all(|p| p[0].is_finite() && p[1].is_finite())
        );
    }
}
