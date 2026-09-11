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
}
