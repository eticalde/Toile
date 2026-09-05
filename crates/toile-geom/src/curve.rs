use kurbo::{CubicBez, ParamCurve, ParamCurveArclen, ParamCurveNearest, Point};

/// How tight the quadrature and the nearest-point solver are asked to run.
///
/// Coordinates are centimetres, so this is a nanometre: far under anything a
/// pattern can express. It costs what it costs because neither function that
/// reads it is on the drape path — `flatten` is, and it consults no
/// tolerance at all.
const ACCURACY: f64 = 1.0e-9;

/// The cubic as `kurbo` sees it.
///
/// The only place the dependency is named. Everything crossing the module
/// boundary is a `[f64; 2]`, so no other crate inherits `kurbo` and a version
/// bump cannot reach past this file.
fn bez(p0: [f64; 2], c1: [f64; 2], c2: [f64; 2], p1: [f64; 2]) -> CubicBez {
    CubicBez::new(
        Point::new(p0[0], p0[1]),
        Point::new(c1[0], c1[1]),
        Point::new(c2[0], c2[1]),
        Point::new(p1[0], p1[1]),
    )
}

/// The cubic as `samples` points, taken at fixed fractions of `t`.
///
/// Exactly `samples` points come back, evaluated at `t = i / samples` for `i`
/// in `0..samples`. `p0` is the first of them and `p1` is **not** among them:
/// the end of a tract is the next node's own anchor, and emitting it here
/// would double every node when a closed contour concatenates its tracts. A
/// `samples` of zero is read as one, so a node always contributes its anchor
/// instead of vanishing from the contour.
///
/// The count answers to `samples` and to nothing else. No tolerance is
/// consulted, so dragging a handle moves the points without ever changing how
/// many there are, and a tangent tweak stays a shape edit instead of turning
/// into a remesh.
pub fn flatten(
    p0: [f64; 2],
    c1: [f64; 2],
    c2: [f64; 2],
    p1: [f64; 2],
    samples: u16,
) -> Vec<[f64; 2]> {
    let n = usize::from(samples.max(1));
    let curve = bez(p0, c1, c2, p1);
    let mut out = Vec::with_capacity(n);
    // The anchor is copied rather than evaluated, so the node lands exactly
    // where the document put it whatever the handles are doing.
    out.push(p0);
    for i in 1..n {
        let p = curve.eval(i as f64 / n as f64);
        out.push([p.x, p.y]);
    }
    out
}

/// The arc length of the cubic, by Gauss-Legendre quadrature.
///
/// This is the true length of the curve, not the length of the polyline
/// `flatten` returns; the two differ by however much the sampling cuts the
/// corners.
pub fn arclen(p0: [f64; 2], c1: [f64; 2], c2: [f64; 2], p1: [f64; 2]) -> f64 {
    bez(p0, c1, c2, p1).arclen(ACCURACY)
}

/// The point of the cubic nearest `q`, as its Bezier parameter and its
/// distance.
///
/// The parameter lands in `[0, 1]` and the distance is in the units of the
/// points. **It is not a fraction of the tract's length**: the two agree only
/// where the control net is symmetric, and on the crotch curve of the block
/// they differ by 1.7 mm — an order of magnitude past what the drawing
/// budgets. A place on a contour is said in arc length along the flattening
/// (`EdgeAnchor.t`), so anything meant for an anchor has to convert; nothing
/// in the tree calls this yet, and this is the note that says which of the
/// two it would be handing over.
pub fn nearest(p0: [f64; 2], c1: [f64; 2], c2: [f64; 2], p1: [f64; 2], q: [f64; 2]) -> (f64, f64) {
    let hit = bez(p0, c1, c2, p1).nearest(Point::new(q[0], q[1]), ACCURACY);
    (hit.t, hit.distance_sq.sqrt())
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "an anchor and a sample count come back exact or not at all"
    )]

    use super::*;

    /// A cubic on the unit circle, from angle `a0` to `a1` in radians.
    ///
    /// The handle length is the textbook `4/3 * tan(theta/4)`, which puts the
    /// curve through both ends with the right tangents there.
    fn arc(a0: f64, a1: f64) -> ([f64; 2], [f64; 2], [f64; 2], [f64; 2]) {
        let k = 4.0 / 3.0 * ((a1 - a0) / 4.0).tan();
        let (p0, p1) = ([a0.cos(), a0.sin()], [a1.cos(), a1.sin()]);
        let c1 = [p0[0] - k * a0.sin(), p0[1] + k * a0.cos()];
        let c2 = [p1[0] + k * a1.sin(), p1[1] - k * a1.cos()];
        (p0, c1, c2, p1)
    }

    /// A curve with both handles pulled well off the chord.
    fn wavy() -> ([f64; 2], [f64; 2], [f64; 2], [f64; 2]) {
        ([0.0, 0.0], [3.0, 8.0], [11.0, -6.0], [14.0, 2.0])
    }

    #[test]
    fn a_straight_cubic_measures_its_chord() {
        // Handles on the thirds of the chord: the cubic is the segment.
        let len = arclen([0.0, 0.0], [1.0, 1.0], [2.0, 2.0], [3.0, 3.0]);
        assert!((len - 3.0 * 2.0_f64.sqrt()).abs() < 1.0e-12, "{len}");
    }

    #[test]
    fn a_quarter_circle_is_within_1e_4_of_the_analytic() {
        // One cubic misses a 90-degree arc by 2.2e-4; two 45-degree tracts
        // miss by 3e-6, and chaining tracts is what a contour does anyway.
        let half = std::f64::consts::FRAC_PI_4;
        let arcs = [arc(0.0, half), arc(half, half * 2.0)];

        let len: f64 = arcs.iter().map(|a| arclen(a.0, a.1, a.2, a.3)).sum();
        assert!(
            (len - std::f64::consts::FRAC_PI_2).abs() < 1.0e-4,
            "length {len}"
        );

        for a in arcs {
            for p in flatten(a.0, a.1, a.2, a.3, 32) {
                let r = p[0].hypot(p[1]);
                assert!((r - 1.0).abs() < 1.0e-4, "radius {r}");
            }
        }
    }

    #[test]
    fn flatten_is_bit_identical_across_runs() {
        let (p0, c1, c2, p1) = wavy();
        let bits = |v: Vec<[f64; 2]>| -> Vec<[u64; 2]> {
            v.iter().map(|p| [p[0].to_bits(), p[1].to_bits()]).collect()
        };
        let first = bits(flatten(p0, c1, c2, p1, 97));
        let second = bits(flatten(p0, c1, c2, p1, 97));
        assert_eq!(first, second);
    }

    #[test]
    fn the_point_count_depends_only_on_the_samples_field() {
        let (p0, c1, c2, p1) = wavy();
        // A tangent tweak: same anchors, handles a third of the way in.
        let tweaked = flatten(p0, [1.0, 2.6], [12.0, -1.3], p1, 40);
        assert_eq!(flatten(p0, c1, c2, p1, 40).len(), 40);
        assert_eq!(tweaked.len(), 40);
        assert_eq!(flatten(p0, c1, c2, p1, 41).len(), 41);
        // A straight cubic is no cheaper than a wild one.
        assert_eq!(flatten(p0, p0, p1, p1, 40).len(), 40);
    }

    #[test]
    fn a_tract_gives_its_anchor_and_stops_short_of_the_next_one() {
        let (p0, c1, c2, p1) = wavy();
        let got = flatten(p0, c1, c2, p1, 8);
        assert_eq!(got[0], p0);
        assert!(got.iter().all(|&p| p != p1));
        assert_eq!(flatten(p0, c1, c2, p1, 0), vec![p0]);
        assert_eq!(flatten(p0, c1, c2, p1, 1), vec![p0]);
    }

    #[test]
    fn nearest_lands_at_the_foot_of_the_perpendicular() {
        let (t, d) = nearest([0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [3.0, 0.0], [1.5, 2.0]);
        assert!((t - 0.5).abs() < 1.0e-6, "t {t}");
        assert!((d - 2.0).abs() < 1.0e-6, "d {d}");
    }

    #[test]
    fn nearest_clamps_to_the_ends_of_the_tract() {
        let (p0, c1, c2, p1) = arc(0.0, std::f64::consts::FRAC_PI_4);
        let (t, _) = nearest(p0, c1, c2, p1, [4.0, -3.0]);
        assert_eq!(t, 0.0);
        let (t, _) = nearest(p0, c1, c2, p1, [0.0, 5.0]);
        assert_eq!(t, 1.0);
    }
}
