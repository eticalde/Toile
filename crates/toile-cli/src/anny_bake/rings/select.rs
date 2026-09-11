use super::geom::{dist, lerp};
use super::intersect::Crossing;

/// The 3D point one crossing evaluates to.
pub(super) fn ring_point(positions: &[[f64; 3]], c: Crossing) -> [f64; 3] {
    let (a, b, t) = c;
    lerp(positions[a as usize], positions[b as usize], t)
}

/// The chord-sum perimeter of a closed loop.
pub(super) fn perimeter(positions: &[[f64; 3]], loop_: &[Crossing]) -> f64 {
    let n = loop_.len();
    (0..n)
        .map(|i| {
            dist(
                ring_point(positions, loop_[i]),
                ring_point(positions, loop_[(i + 1) % n]),
            )
        })
        .sum()
}

/// The mean of a loop's points.
pub(super) fn centroid(positions: &[[f64; 3]], loop_: &[Crossing]) -> [f64; 3] {
    let mut sum = [0.0; 3];
    for &c in loop_ {
        let p = ring_point(positions, c);
        sum[0] += p[0];
        sum[1] += p[1];
        sum[2] += p[2];
    }
    let n = loop_.len() as f64;
    [sum[0] / n, sum[1] / n, sum[2] / n]
}

/// A loop's left-right extent: `max(x) - min(x)`.
pub(super) fn x_extent(positions: &[[f64; 3]], loop_: &[Crossing]) -> f64 {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for &c in loop_ {
        let x = ring_point(positions, c)[0];
        lo = lo.min(x);
        hi = hi.max(x);
    }
    hi - lo
}

/// Half of [`x_extent`]: how far a loop reaches from the body's own x = 0
/// axis, on its wider side.
fn half_width(positions: &[[f64; 3]], loop_: &[Crossing]) -> f64 {
    x_extent(positions, loop_) / 2.0
}

/// A cross-section wider than this on one side is not a trunk or head ring
/// any more — it is the mesh's own topology fusing the torso to a limb (the
/// arms meet the chest above the underarm; the legs meet the pelvis below
/// the fork), and a loop that wide is picking up that fused blob rather
/// than a real girth. Chosen from this mesh's own data: a genuine chest or
/// hip half-width never exceeds about 0.18 m, while every fused case
/// measured over 0.23 m.
const MAX_TRUNK_HALF_WIDTH_M: f64 = 0.20;

/// The trunk/head selection rule: among the loops that straddle the body's
/// own x = 0 axis (rather than sitting entirely on one side, as a limb's
/// loop does) and are not implausibly wide (see [`MAX_TRUNK_HALF_WIDTH_M`]),
/// the longest.
///
/// Returns `None` if no loop qualifies — the caller's height is not a valid
/// trunk cut (below the leg fork, or fused with a limb).
pub(super) fn pick_trunk_loop(
    positions: &[[f64; 3]],
    loops: &[Vec<Crossing>],
) -> Option<Vec<Crossing>> {
    const EPS: f64 = 1.0e-6;
    let mut best: Option<(&Vec<Crossing>, f64)> = None;
    for candidate in loops {
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for &c in candidate {
            let x = ring_point(positions, c)[0];
            lo = lo.min(x);
            hi = hi.max(x);
        }
        let straddles = lo < -EPS && hi > EPS;
        if !straddles || half_width(positions, candidate) >= MAX_TRUNK_HALF_WIDTH_M {
            continue;
        }
        let per = perimeter(positions, candidate);
        if best.is_none_or(|(_, best_per)| per > best_per) {
            best = Some((candidate, per));
        }
    }
    best.map(|(l, _)| l.clone())
}

/// The loop whose centroid sits nearest `target`, with no other filter.
///
/// Used only where the cutting plane has no sibling structure nearby to be
/// confused with (the neck, cut on its own axis, crosses nothing else) —
/// see [`pick_limb_loop`] for the paired-limb case, which additionally
/// guards against a fused reading.
pub(super) fn pick_nearest_loop(
    positions: &[[f64; 3]],
    loops: &[Vec<Crossing>],
    target: [f64; 3],
) -> Option<Vec<Crossing>> {
    let mut best: Option<(&Vec<Crossing>, f64)> = None;
    for candidate in loops {
        let d = dist(centroid(positions, candidate), target);
        if best.is_none_or(|(_, best_d)| d < best_d) {
            best = Some((candidate, d));
        }
    }
    best.map(|(l, _)| l.clone())
}

/// The limb selection rule: the loop nearest `target`, among loops that
/// prove the plane still separates this limb from its sibling structure.
///
/// A limb's own cross-section always has a sibling loop nearby — the other
/// leg or arm, or (near a joint) the torso itself — so seeing only one loop
/// total means the plane has fused with that sibling rather than cutting
/// the limb cleanly (the same fusion [`pick_trunk_loop`] rejects by width;
/// a limb ring rejects it by loop count instead, since a fused limb loop's
/// centroid can otherwise land deceptively close to `target`).
pub(super) fn pick_limb_loop(
    positions: &[[f64; 3]],
    loops: &[Vec<Crossing>],
    target: [f64; 3],
) -> Option<Vec<Crossing>> {
    if loops.len() < 2 {
        return None;
    }
    pick_nearest_loop(positions, loops, target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_loop() -> (Vec<[f64; 3]>, Vec<Crossing>) {
        let positions = vec![
            [-1.0, 0.0, -1.0],
            [1.0, 0.0, -1.0],
            [1.0, 0.0, 1.0],
            [-1.0, 0.0, 1.0],
        ];
        let loop_ = vec![(0, 0, 0.0), (1, 1, 0.0), (2, 2, 0.0), (3, 3, 0.0)];
        (positions, loop_)
    }

    #[test]
    fn perimeter_of_a_two_by_two_square_is_eight() {
        let (positions, loop_) = square_loop();
        assert!((perimeter(&positions, &loop_) - 8.0).abs() < 1.0e-9);
    }

    #[test]
    fn centroid_of_a_centred_square_is_the_origin() {
        let (positions, loop_) = square_loop();
        let c = centroid(&positions, &loop_);
        assert!(dist(c, [0.0, 0.0, 0.0]) < 1.0e-9);
    }

    #[test]
    fn pick_trunk_loop_rejects_a_loop_entirely_on_one_side() {
        let positions = vec![
            [1.0, 0.0, -1.0],
            [3.0, 0.0, -1.0],
            [3.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
        ];
        let arm_loop = vec![(0, 0, 0.0), (1, 1, 0.0), (2, 2, 0.0), (3, 3, 0.0)];
        assert!(pick_trunk_loop(&positions, &[arm_loop]).is_none());
    }

    #[test]
    fn pick_trunk_loop_takes_the_longer_of_two_straddling_loops() {
        // Both squares straddle x = 0 and stay well under the trunk
        // half-width bound (0.20 m): vertices 0..4 are a small square
        // (half-width 0.05 m), 4..8 a bigger one (half-width 0.15 m) with a
        // longer perimeter.
        let positions = vec![
            [-0.05, 0.0, -0.05],
            [0.05, 0.0, -0.05],
            [0.05, 0.0, 0.05],
            [-0.05, 0.0, 0.05],
            [-0.15, 0.0, -0.15],
            [0.15, 0.0, -0.15],
            [0.15, 0.0, 0.15],
            [-0.15, 0.0, 0.15],
        ];
        let small = vec![(0, 0, 0.0), (1, 1, 0.0), (2, 2, 0.0), (3, 3, 0.0)];
        let big = vec![(4, 4, 0.0), (5, 5, 0.0), (6, 6, 0.0), (7, 7, 0.0)];
        let picked = pick_trunk_loop(&positions, &[small, big.clone()]).unwrap();
        assert_eq!(picked, big);
    }

    #[test]
    fn pick_limb_loop_needs_at_least_two_loops() {
        let (positions, one) = square_loop();
        assert!(pick_limb_loop(&positions, &[one], [0.0, 0.0, 0.0]).is_none());
    }

    #[test]
    fn pick_limb_loop_takes_the_nearer_of_two() {
        // Vertices 0..4 form a square at the origin; 4..8 the same square
        // shifted 100 m away, so the two loops' centroids are unmistakably
        // near and far from the target.
        let (near_positions, _) = square_loop();
        let mut positions = near_positions.clone();
        positions.extend(near_positions.iter().map(|p| [p[0] + 100.0, p[1], p[2]]));
        let near = vec![(0, 0, 0.0), (1, 1, 0.0), (2, 2, 0.0), (3, 3, 0.0)];
        let far = vec![(4, 4, 0.0), (5, 5, 0.0), (6, 6, 0.0), (7, 7, 0.0)];
        let loops = vec![far.clone(), near.clone()];
        let picked = pick_limb_loop(&positions, &loops, [0.0, 0.0, 0.0]).unwrap();
        assert_eq!(picked, near);
    }
}
