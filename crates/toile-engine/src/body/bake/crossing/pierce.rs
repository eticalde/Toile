/// A point in the mesh's own frame, in metres.
pub(super) type Point = [f64; 3];

/// Whether two triangles that share no vertex pass through each other.
///
/// Two sheets that cross meet along a segment, and each end of that segment
/// is an edge of one triangle going through the inside of the other. So six
/// edges are asked, three of each against the other triangle, and one yes is
/// enough.
///
/// Every comparison is strict. Two triangles lying in one plane, or an edge
/// that only grazes the other triangle's rim, are a surface touching itself
/// and not passing through, and they read as no crossing.
pub(super) fn crosses(p: [Point; 3], q: [Point; 3]) -> bool {
    (0..3).any(|k| {
        let next = (k + 1) % 3;
        pierces([p[k], p[next]], q) || pierces([q[k], q[next]], p)
    })
}

/// Whether a segment goes through the inside of a triangle.
///
/// All of it is the sign of a volume. The two ends have to sit on opposite
/// sides of the triangle's plane, and then the line through them has to pass
/// the three edges all the same way round, which is what being inside the
/// three of them means. No point of intersection is ever computed, so there
/// is no division here to go wrong on a sliver.
fn pierces(s: [Point; 2], t: [Point; 3]) -> bool {
    let (from, to) = (
        volume(t[0], t[1], t[2], s[0]),
        volume(t[0], t[1], t[2], s[1]),
    );
    if !((from > 0.0 && to < 0.0) || (from < 0.0 && to > 0.0)) {
        return false;
    }
    let round = [0, 1, 2].map(|k| volume(s[0], s[1], t[k], t[(k + 1) % 3]));
    round.iter().all(|&v| v > 0.0) || round.iter().all(|&v| v < 0.0)
}

/// Six times the signed volume of the tetrahedron `a b c d`: positive when
/// `d` is on the side `a b c` is seen wound counter-clockwise from.
fn volume(a: Point, b: Point, c: Point, d: Point) -> f64 {
    let [u, v, w] = [b, c, d].map(|p| [0, 1, 2].map(|k| p[k] - a[k]));
    u[0] * (v[1] * w[2] - v[2] * w[1])
        + u[1] * (v[2] * w[0] - v[0] * w[2])
        + u[2] * (v[0] * w[1] - v[1] * w[0])
}
