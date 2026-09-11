/// Splits every quad into two triangles on its shorter diagonal, compared as
/// squared length in f64 so no square root is needed to pick a side.
///
/// A convex, roughly planar quad wound CCW keeps that winding under either
/// diagonal split — `(v0,v1,v2),(v0,v2,v3)` for the `v0`-`v2` diagonal,
/// `(v0,v1,v3),(v1,v2,v3)` for the other — which is why the OBJ file's own
/// loop order is used directly, with no repair pass and no dependency on the
/// quad's exact planarity.
pub fn triangulate(quads: &[[u32; 4]], positions: &[[f64; 3]]) -> Vec<u32> {
    let mut out = Vec::with_capacity(quads.len() * 6);
    for &[i0, i1, i2, i3] in quads {
        let p = |i: u32| positions[i as usize];
        if dist_sq(p(i0), p(i2)) <= dist_sq(p(i1), p(i3)) {
            out.extend_from_slice(&[i0, i1, i2, i0, i2, i3]);
        } else {
            out.extend_from_slice(&[i0, i1, i3, i1, i2, i3]);
        }
    }
    out
}

fn dist_sq(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_a_square_on_its_shorter_diagonal() {
        // A unit square: both diagonals are equal, so the tie takes v0-v2.
        let square = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ];
        let tris = triangulate(&[[0, 1, 2, 3]], &square);
        assert_eq!(tris, vec![0, 1, 2, 0, 2, 3]);
    }

    #[test]
    fn takes_the_v1_v3_diagonal_when_it_is_shorter() {
        // A quad stretched so the v1-v3 diagonal is the short one.
        let kite = [
            [0.0, 0.0, 0.0],
            [1.0, 0.1, 0.0],
            [4.0, 0.0, 0.0],
            [1.0, -0.1, 0.0],
        ];
        let tris = triangulate(&[[0, 1, 2, 3]], &kite);
        assert_eq!(tris, vec![0, 1, 3, 1, 2, 3]);
    }
}
