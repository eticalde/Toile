/// The arc length from the first node to each node, and round to it again.
///
/// `n + 1` entries for `n` points: the first is zero, the last is the
/// perimeter, and the tract leaving node `i` measures `cum[i + 1] - cum[i]`.
///
/// This is the one place a contour is measured. The number a person reads off
/// the drawing and the table the sampler walks come out of the same sum, in
/// the same order, so they cannot drift apart.
pub fn cumulative(contour: &[[f64; 2]]) -> Vec<f64> {
    let n = contour.len();
    let mut cum = Vec::with_capacity(n + 1);
    let mut total = 0.0;
    cum.push(0.0);
    for i in 0..n {
        let (a, b) = (contour[i], contour[(i + 1) % n]);
        total += ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        cum.push(total);
    }
    cum
}

/// The length of the whole closed contour.
pub fn perimeter(contour: &[[f64; 2]]) -> f64 {
    cumulative(contour).last().copied().unwrap_or_default()
}

/// The length of the walk from node `from` forward to node `to`.
///
/// The walk runs the way the contour does and passes the closure when it has
/// to, so `run_length(c, 3, 1)` is the long way round. Indices are taken
/// around the closure; an empty contour measures zero.
pub fn run_length(contour: &[[f64; 2]], from: usize, to: usize) -> f64 {
    let n = contour.len();
    if n == 0 {
        return 0.0;
    }
    let cum = cumulative(contour);
    let (from, to) = (from % n, to % n);
    if to >= from {
        cum[to] - cum[from]
    } else {
        cum[n] - cum[from] + cum[to]
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "the corners of a unit square measure exactly"
    )]

    use super::*;

    /// A unit square, corners only.
    fn square() -> Vec<[f64; 2]> {
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]
    }

    #[test]
    fn an_empty_contour_measures_nothing() {
        assert_eq!(cumulative(&[]), vec![0.0]);
        assert_eq!(perimeter(&[]), 0.0);
        assert_eq!(run_length(&[], 0, 0), 0.0);
    }

    #[test]
    fn a_lone_point_closes_on_itself() {
        assert_eq!(cumulative(&[[3.0, 4.0]]), vec![0.0, 0.0]);
    }

    #[test]
    fn cumulative_matches_the_sum_of_the_edges() {
        let cum = cumulative(&square());
        assert_eq!(cum, vec![0.0, 1.0, 2.0, 3.0, 4.0]);
        assert_eq!(perimeter(&square()), 4.0);
    }

    #[test]
    fn a_run_that_passes_the_closure_keeps_going() {
        assert_eq!(run_length(&square(), 1, 3), 2.0);
        assert_eq!(run_length(&square(), 3, 1), 2.0);
        assert_eq!(run_length(&square(), 2, 2), 0.0);
    }

    #[test]
    fn an_index_past_the_end_is_taken_around_the_closure() {
        assert_eq!(run_length(&square(), 4, 6), run_length(&square(), 0, 2));
    }

    #[test]
    fn a_degenerate_edge_adds_nothing() {
        let dup = vec![[0.0, 0.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];
        assert_eq!(cumulative(&dup)[2], 1.0);
    }
}
