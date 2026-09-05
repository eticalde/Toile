use thiserror::Error;

/// How close two points may come before they count as one, in metres.
const EPS: f64 = 1.0e-6;

/// What stops a contour from being a simple closed polygon.
///
/// Every variant names the node a person can go and look at, because this is
/// what the drawing paints in red and the status bar spells out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ContourFault {
    /// A coordinate that is infinite or not a number.
    #[error("node {at} is not at a finite coordinate")]
    NotFinite {
        /// The offending node.
        at: usize,
    },
    /// Two nodes within a micrometre of each other.
    #[error("nodes {a} and {b} sit on top of each other")]
    Coincident {
        /// The earlier node.
        a: usize,
        /// The later one.
        b: usize,
    },
    /// Two tracts that cross, which no cut piece of cloth can do.
    #[error("the tract leaving node {i} crosses the one leaving node {j}")]
    SelfIntersects {
        /// The node the first tract leaves.
        i: usize,
        /// The node the second tract leaves.
        j: usize,
    },
    /// Fewer than three nodes, or three that enclose no area.
    #[error("the contour encloses nothing")]
    Degenerate,
}

/// Checks that `contour` is a simple closed polygon the mesher can take.
///
/// Run on every resolution, before anything reaches the mesher: degenerate
/// geometry is caught where the person made it, not deep in a triangulation.
///
/// The input is the flattening and not the nodes, so its length is the sum of
/// the tracts' sample counts rather than the count of nodes: hundreds of
/// points for a piece, not tens. The quadratic scans are right at that size
/// and no larger, which is why the document bounds what a tract may be
/// flattened at instead of letting a file name the number.
///
/// # Errors
/// The first fault found, in the order a person would want to hear them: a
/// coordinate that is not a number, then a contour too small to enclose
/// anything, then nodes on top of each other, then a contour with no width,
/// then tracts that cross.
pub fn check_closed(contour: &[[f64; 2]]) -> Result<(), ContourFault> {
    for (at, p) in contour.iter().enumerate() {
        if !p[0].is_finite() || !p[1].is_finite() {
            return Err(ContourFault::NotFinite { at });
        }
    }
    let n = contour.len();
    if n < 3 {
        return Err(ContourFault::Degenerate);
    }
    for a in 0..n {
        for b in (a + 1)..n {
            if distance2(contour[a], contour[b]) < EPS * EPS {
                return Err(ContourFault::Coincident { a, b });
            }
        }
    }
    // A contour with no width overlaps itself everywhere, so it is answered
    // for what it is before the crossing scan can call it something else.
    if contour
        .iter()
        .all(|&p| side(contour[0], contour[1], p) == 0.0)
    {
        return Err(ContourFault::Degenerate);
    }
    for i in 0..n {
        for j in (i + 1)..n {
            if !adjacent(i, j, n) && crosses(tract(contour, i), tract(contour, j)) {
                return Err(ContourFault::SelfIntersects { i, j });
            }
        }
    }
    if signed_area(contour).abs() < EPS * EPS {
        return Err(ContourFault::Degenerate);
    }
    Ok(())
}

/// Twice the area the contour encloses, negative one way round and positive
/// the other.
pub fn signed_area(contour: &[[f64; 2]]) -> f64 {
    let n = contour.len();
    let mut sum = 0.0;
    for i in 0..n {
        let (a, b) = (contour[i], contour[(i + 1) % n]);
        sum += a[0] * b[1] - b[0] * a[1];
    }
    sum / 2.0
}

/// The tract leaving node `i`, closing onto the first node from the last.
fn tract(contour: &[[f64; 2]], i: usize) -> ([f64; 2], [f64; 2]) {
    (contour[i], contour[(i + 1) % contour.len()])
}

/// Whether two tracts share a node, which is not a crossing.
fn adjacent(i: usize, j: usize, n: usize) -> bool {
    j == i + 1 || (i == 0 && j + 1 == n)
}

fn distance2(a: [f64; 2], b: [f64; 2]) -> f64 {
    (b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)
}

/// Where `b` falls relative to the ray from `o` through `a`: positive to one
/// side, negative to the other, zero when the three are in line.
fn side(o: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
}

/// Whether `p`, already known to be in line with the tract, lies on it.
fn within(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> bool {
    p[0] >= a[0].min(b[0])
        && p[0] <= a[0].max(b[0])
        && p[1] >= a[1].min(b[1])
        && p[1] <= a[1].max(b[1])
}

/// Whether two tracts that share no node meet anywhere.
fn crosses(first: ([f64; 2], [f64; 2]), second: ([f64; 2], [f64; 2])) -> bool {
    let (a, b) = first;
    let (c, d) = second;
    let (s1, s2) = (side(c, d, a), side(c, d, b));
    let (s3, s4) = (side(a, b, c), side(a, b, d));
    if ((s1 > 0.0) != (s2 > 0.0)) && ((s3 > 0.0) != (s4 > 0.0)) && s1 != 0.0 && s2 != 0.0 {
        return true;
    }
    (s1 == 0.0 && within(c, d, a))
        || (s2 == 0.0 && within(c, d, b))
        || (s3 == 0.0 && within(a, b, c))
        || (s4 == 0.0 && within(a, b, d))
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "the corners of a unit square measure exactly"
    )]

    use super::*;

    fn square() -> Vec<[f64; 2]> {
        vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]
    }

    #[test]
    fn a_square_is_a_contour() {
        assert_eq!(check_closed(&square()), Ok(()));
    }

    #[test]
    fn a_nan_is_rejected_before_the_mesher() {
        let mut c = square();
        c[2][1] = f64::NAN;
        assert_eq!(check_closed(&c), Err(ContourFault::NotFinite { at: 2 }));
        c[2][1] = f64::INFINITY;
        assert_eq!(check_closed(&c), Err(ContourFault::NotFinite { at: 2 }));
    }

    #[test]
    fn fewer_than_three_nodes_enclose_nothing() {
        assert_eq!(check_closed(&[]), Err(ContourFault::Degenerate));
        assert_eq!(
            check_closed(&[[0.0, 0.0], [1.0, 0.0]]),
            Err(ContourFault::Degenerate)
        );
    }

    #[test]
    fn three_nodes_in_line_enclose_nothing() {
        let line = [[0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [3.0, 0.0]];
        assert_eq!(check_closed(&line), Err(ContourFault::Degenerate));
    }

    #[test]
    fn two_nodes_a_micrometre_apart_are_one_node() {
        let mut c = square();
        c[3] = [c[1][0] + 1.0e-9, c[1][1]];
        assert_eq!(
            check_closed(&c),
            Err(ContourFault::Coincident { a: 1, b: 3 })
        );
    }

    #[test]
    fn a_self_intersecting_contour_is_rejected() {
        let bowtie = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        assert_eq!(
            check_closed(&bowtie),
            Err(ContourFault::SelfIntersects { i: 1, j: 3 })
        );
    }

    #[test]
    fn a_tract_that_only_touches_another_is_still_a_crossing() {
        let touching = [[0.0, 0.0], [2.0, 0.0], [1.0, 1.0], [1.0, 0.0], [0.0, 2.0]];
        assert!(matches!(
            check_closed(&touching),
            Err(ContourFault::SelfIntersects { .. })
        ));
    }

    #[test]
    fn winding_shows_in_the_sign_of_the_area() {
        assert_eq!(signed_area(&square()), 1.0);
        let mut back = square();
        back.reverse();
        assert_eq!(signed_area(&back), -1.0);
    }
}
