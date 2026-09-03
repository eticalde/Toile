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
pub fn sample_closed(contour: &[[f64; 2]], fractions: &[f64]) -> Vec<[f64; 2]> {
    let n = contour.len();
    let mut cum = Vec::with_capacity(n + 1);
    let mut total = 0.0;
    cum.push(0.0);
    for i in 0..n {
        let (a, b) = (contour[i], contour[(i + 1) % n]);
        total += ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        cum.push(total);
    }

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
