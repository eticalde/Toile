/// Small `f64` vector helpers shared by every step of the ring bake: plane
/// arithmetic, loop assembly and loop selection all reduce to these.
pub(super) fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub(super) fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub(super) fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

pub(super) fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(super) fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    dot(sub(a, b), sub(a, b)).sqrt()
}

/// A unit vector along `a`.
///
/// # Panics
/// If `a` is (near) the zero vector: every caller builds `a` from two
/// distinct joint centroids, which are never that close together.
pub(super) fn unit(a: [f64; 3]) -> [f64; 3] {
    let len = dot(a, a).sqrt();
    assert!(len > 1.0e-9, "cannot normalize a near-zero vector {a:?}");
    scale(a, 1.0 / len)
}

pub(super) fn lerp(a: [f64; 3], b: [f64; 3], t: f64) -> [f64; 3] {
    add(a, scale(sub(b, a), t))
}

/// The index of the body vertex nearest `target`.
///
/// Used only where a plane cannot cut a clean ring at all (the true
/// shoulder joint, fused into the torso — see `crate::anny_bake`'s doc):
/// the nearest single vertex is the best surface proxy for a joint that
/// has no cross-section of its own to measure.
pub(super) fn nearest_vertex(positions: &[[f64; 3]], target: [f64; 3]) -> u32 {
    let mut best = 0usize;
    let mut best_d = f64::INFINITY;
    for (i, &p) in positions.iter().enumerate() {
        let d = dist(p, target);
        if d < best_d {
            best_d = d;
            best = i;
        }
    }
    best as u32
}

/// The index of the highest (greatest `y`) body vertex within `radius` of
/// `centre`.
///
/// Used for the acromion (see `crate::anny_bake`'s doc): the shoulder's
/// bony top has no joint of its own marking it (`joint-r-shoulder` marks
/// the ball joint, inside the arm), but it is exactly the topmost point of
/// the deltoid cap over that joint, findable directly on the mesh surface.
///
/// # Panics
/// If no vertex lies within `radius` of `centre`: a bake-time placement
/// mistake (too small a radius), not a data problem.
pub(super) fn topmost_vertex_within(positions: &[[f64; 3]], centre: [f64; 3], radius: f64) -> u32 {
    let mut best: Option<(usize, f64)> = None;
    for (i, &p) in positions.iter().enumerate() {
        if dist(p, centre) > radius {
            continue;
        }
        if best.is_none_or(|(_, best_y)| p[1] > best_y) {
            best = Some((i, p[1]));
        }
    }
    best.map_or_else(
        || panic!("no vertex within {radius} m of {centre:?}"),
        |(i, _)| i as u32,
    )
}

/// The index of the most posterior (smallest `z`) body vertex whose height
/// falls in `[y_lo, y_hi]` and whose distance from the sagittal midline
/// (`|x|`) is at most `x_abs_max`.
///
/// Used for the nape (the C7 vertebra, at the back of the base of the
/// neck): unlike the acromion, there is no nearby joint to search around
/// at all, but the nape is still a findable surface feature — the most
/// posterior point of the back, restricted to the neck-to-shoulder height
/// band and to the midline, so it cannot wander onto a shoulder blade.
///
/// # Panics
/// If no vertex satisfies both bounds: a bake-time placement mistake in
/// the band or the midline tolerance, not a data problem.
pub(super) fn most_posterior_in_band(
    positions: &[[f64; 3]],
    y_lo: f64,
    y_hi: f64,
    x_abs_max: f64,
) -> u32 {
    let mut best: Option<(usize, f64)> = None;
    for (i, &p) in positions.iter().enumerate() {
        if p[1] < y_lo || p[1] > y_hi || p[0].abs() > x_abs_max {
            continue;
        }
        if best.is_none_or(|(_, best_z)| p[2] < best_z) {
            best = Some((i, p[2]));
        }
    }
    best.map_or_else(
        || panic!("no vertex in y=[{y_lo}, {y_hi}], |x| <= {x_abs_max}"),
        |(i, _)| i as u32,
    )
}

/// A named joint's centroid, by its full `joint-*` group name.
///
/// # Panics
/// If no group of that name is among `joints`: every name this bake asks
/// for is one of the 125 joint-cube groups `groups::joint_centroids` reads,
/// so a miss here means a typo in this module, not a data problem.
pub(super) fn joint(joints: &[(String, [f64; 3])], name: &str) -> [f64; 3] {
    joints
        .iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no joint group named `{name}`"))
        .1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "exact literal inputs lerp to an exact literal result"
    )]
    fn lerp_at_zero_and_one_lands_on_the_endpoints() {
        let a = [0.0, 1.0, 2.0];
        let b = [4.0, 5.0, 6.0];
        assert_eq!(lerp(a, b, 0.0), a);
        assert_eq!(lerp(a, b, 1.0), b);
        assert_eq!(lerp(a, b, 0.5), [2.0, 3.0, 4.0]);
    }

    #[test]
    fn unit_normalizes_to_length_one() {
        let u = unit([3.0, 4.0, 0.0]);
        assert!((dist(u, [0.0, 0.0, 0.0]) - 1.0).abs() < 1.0e-12);
    }

    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "exact literal input round-trips through a plain lookup"
    )]
    fn joint_finds_the_named_group() {
        let joints = vec![("joint-neck".to_owned(), [1.0, 2.0, 3.0])];
        assert_eq!(joint(&joints, "joint-neck"), [1.0, 2.0, 3.0]);
    }

    #[test]
    fn nearest_vertex_picks_the_closest_point() {
        let positions = vec![[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [1.0, 1.0, 1.0]];
        assert_eq!(nearest_vertex(&positions, [1.1, 1.1, 1.1]), 2);
        assert_eq!(nearest_vertex(&positions, [9.0, 0.0, 0.0]), 1);
    }

    #[test]
    fn topmost_vertex_within_ignores_a_higher_point_outside_the_radius() {
        let positions = vec![
            [0.0, 0.15, 0.0],  // within radius, the higher of the two nearby
            [0.05, 0.10, 0.0], // within radius, lower
            [0.0, 100.0, 0.0], // far away and much higher: must be excluded
        ];
        assert_eq!(topmost_vertex_within(&positions, [0.0, 0.0, 0.0], 0.2), 0);
    }

    #[test]
    #[should_panic(expected = "no vertex within")]
    fn topmost_vertex_within_panics_if_nothing_is_close_enough() {
        let positions = vec![[10.0, 10.0, 10.0]];
        topmost_vertex_within(&positions, [0.0, 0.0, 0.0], 0.1);
    }

    #[test]
    fn most_posterior_in_band_respects_both_the_height_and_the_midline() {
        let positions = vec![
            [0.0, 0.5, -0.2],  // in band, on the midline: the answer
            [0.0, 0.9, -0.5],  // more posterior, but off the height band
            [0.10, 0.5, -0.9], // most posterior of all, but off the midline
        ];
        assert_eq!(most_posterior_in_band(&positions, 0.4, 0.6, 0.05), 0);
    }

    #[test]
    #[should_panic(expected = "no vertex in")]
    fn most_posterior_in_band_panics_if_nothing_qualifies() {
        let positions = vec![[0.0, 0.0, 0.0]];
        most_posterior_in_band(&positions, 1.0, 2.0, 0.01);
    }
}
