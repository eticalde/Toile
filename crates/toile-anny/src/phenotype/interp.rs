/// `PyTorch`'s `torch.linspace(start, end, n)` for `f64`, replicated bit for
/// bit rather than approximated: the first half of the array is computed
/// forward from `start` and the second half backward from `end`, which is
/// the numerically-stabler kernel real torch ships. The naive single formula
/// `start + i * (end - start) / (n - 1)` looks equivalent but is not
/// bit-identical — for `age`'s anchors (`start = -1/3`) it gives element 2
/// as the plain `1.0 / 3.0`, while torch's own output is one ULP above that
/// (`0.333_333_333_333_333_37`), which is the value this function reproduces
/// and the age-anchor test below pins.
pub fn linspace(start: f64, end: f64, n: usize) -> Vec<f64> {
    if n == 1 {
        return vec![start];
    }
    let step = (end - start) / ((n - 1) as f64);
    let halfway = n / 2;
    (0..n)
        .map(|i| {
            if i < halfway {
                start + step * (i as f64)
            } else {
                end - step * ((n - 1 - i) as f64)
            }
        })
        .collect()
}

/// Linear interpolation coefficients over `anchors` (ascending) for a
/// value `v`, mirroring Anny's `linear_interpolation_coefficients` with
/// `extrapolate=False`: a `searchsorted(side="left")` locates the bracket,
/// then the two bracketing anchors split a weight of 1 between them
/// (clamped, so a value outside the anchor range saturates at the nearest
/// end rather than extrapolating).
///
/// Returns one coefficient per anchor; all zero except at most two adjacent
/// entries, which sum to 1.
pub fn coefficients(v: f64, anchors: &[f64]) -> Vec<f64> {
    let n = anchors.len();
    let idx = anchors.iter().position(|&a| a >= v).unwrap_or(n);
    let idx = idx.clamp(1, n - 1);
    let (lo, hi) = (anchors[idx - 1], anchors[idx]);
    let alpha = ((v - lo) / (hi - lo)).clamp(0.0, 1.0);
    let mut out = vec![0.0; n];
    out[idx - 1] = 1.0 - alpha;
    out[idx] = alpha;
    out
}

/// Race is not interpolated over anchors: each of the three values is
/// weighted by its own share of the sum, defaulting to an equal third when
/// the sum is zero (mirroring Anny's `nan_to_num` fallback on the same
/// formula).
pub fn race_weights(african: f64, asian: f64, caucasian: f64) -> [f64; 3] {
    let sum = african + asian + caucasian;
    if sum == 0.0 {
        [1.0 / 3.0; 3]
    } else {
        [african / sum, asian / sum, caucasian / sum]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "the exact bit pattern is the point of this test"
    )]
    fn age_anchor_two_carries_the_one_ulp_asymmetry() {
        let anchors = linspace(-1.0 / 3.0, 1.0, 5);
        assert_eq!(anchors[2], 0.333_333_333_333_333_37);
        assert_eq!(anchors[0], -1.0 / 3.0);
        assert_eq!(anchors[4], 1.0);
    }

    #[test]
    fn trivial_anchors_land_exactly_on_their_literals() {
        assert_eq!(linspace(0.0, 1.0, 2), vec![0.0, 1.0]);
        assert_eq!(linspace(0.0, 1.0, 3), vec![0.0, 0.5, 1.0]);
    }

    #[test]
    fn worked_coefficient_checks_over_three_anchors() {
        let anchors = [0.0, 0.5, 1.0];
        assert_eq!(coefficients(0.5, &anchors), vec![0.0, 1.0, 0.0]);
        assert_eq!(coefficients(0.0, &anchors), vec![1.0, 0.0, 0.0]);
        assert_eq!(coefficients(1.0, &anchors), vec![0.0, 0.0, 1.0]);
    }

    #[test]
    fn coefficients_saturate_past_either_end() {
        let anchors = [0.0, 0.5, 1.0];
        assert_eq!(coefficients(-5.0, &anchors), vec![1.0, 0.0, 0.0]);
        assert_eq!(coefficients(5.0, &anchors), vec![0.0, 0.0, 1.0]);
    }

    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "exact literal thirds over a hand-built input"
    )]
    fn race_defaults_split_exactly_in_thirds() {
        let w = race_weights(0.5, 0.5, 0.5);
        assert_eq!(w, [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0]);
    }

    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "exact literal thirds over a hand-built input"
    )]
    fn race_falls_back_to_thirds_when_the_sum_is_zero() {
        assert_eq!(
            race_weights(0.0, 0.0, 0.0),
            [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0]
        );
    }
}
