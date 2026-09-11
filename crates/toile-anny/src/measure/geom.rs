use crate::asset::RingPoint;

/// The 3D point a baked ring point evaluates to against `positions`: a fixed
/// fraction along a fixed mesh edge. This is the only place a ring ever
/// touches a mesh, so every measurement in `crate::measure` is built on it.
pub(crate) fn at(positions: &[f32], p: RingPoint) -> [f32; 3] {
    let a = p.vertex_a as usize * 3;
    let b = p.vertex_b as usize * 3;
    [
        positions[a] + p.t * (positions[b] - positions[a]),
        positions[a + 1] + p.t * (positions[b + 1] - positions[a + 1]),
        positions[a + 2] + p.t * (positions[b + 2] - positions[a + 2]),
    ]
}

/// Euclidean distance between two points, in whatever unit `a` and `b` carry
/// (metres, for the shipped asset).
pub(crate) fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// The sum of consecutive chord lengths around a closed ring: a girth, in
/// whatever unit `positions` carries.
///
/// `ring` is a closed loop — the last point connects back to the first —
/// which is why this sums over `ring.len()` chords rather than
/// `ring.len() - 1`.
pub(crate) fn perimeter(positions: &[f32], ring: &[RingPoint]) -> f32 {
    let n = ring.len();
    (0..n)
        .map(|i| dist(at(positions, ring[i]), at(positions, ring[(i + 1) % n])))
        .sum()
}

/// A ring's centroid: the mean of its points, in whatever unit `positions`
/// carries. Used as a landmark for the length measurements — since every
/// point it averages moves with the morphed body, so does the centroid.
pub(crate) fn centroid(positions: &[f32], ring: &[RingPoint]) -> [f32; 3] {
    let mut sum = [0.0f32; 3];
    for &p in ring {
        let q = at(positions, p);
        sum[0] += q[0];
        sum[1] += q[1];
        sum[2] += q[2];
    }
    let n = ring.len() as f32;
    [sum[0] / n, sum[1] / n, sum[2] / n]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_ring() -> (Vec<f32>, Vec<RingPoint>) {
        // A unit square's four corners, each stored as a ring point sitting
        // exactly on a mesh vertex (t = 0) so the geometry is easy to check
        // by hand: perimeter 4, centroid at the square's own centre.
        let positions = vec![
            0.0, 0.0, 0.0, // vertex 0
            1.0, 0.0, 0.0, // vertex 1
            1.0, 0.0, 1.0, // vertex 2
            0.0, 0.0, 1.0, // vertex 3
        ];
        let ring = (0..4)
            .map(|i| RingPoint {
                vertex_a: i,
                vertex_b: i,
                t: 0.0,
            })
            .collect();
        (positions, ring)
    }

    #[test]
    fn perimeter_of_a_unit_square_is_four() {
        let (positions, ring) = square_ring();
        assert!((perimeter(&positions, &ring) - 4.0).abs() < 1.0e-6);
    }

    #[test]
    fn centroid_of_a_unit_square_is_its_centre() {
        let (positions, ring) = square_ring();
        let c = centroid(&positions, &ring);
        assert!((c[0] - 0.5).abs() < 1.0e-6);
        assert!((c[1] - 0.0).abs() < 1.0e-6);
        assert!((c[2] - 0.5).abs() < 1.0e-6);
    }

    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "exact literal inputs lerp to an exact literal result"
    )]
    fn at_lerps_between_the_two_named_vertices() {
        let positions = vec![0.0, 0.0, 0.0, 4.0, 8.0, 0.0];
        let p = RingPoint {
            vertex_a: 0,
            vertex_b: 1,
            t: 0.25,
        };
        assert_eq!(at(&positions, p), [1.0, 2.0, 0.0]);
    }
}
