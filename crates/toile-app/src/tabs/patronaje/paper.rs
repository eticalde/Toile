/// The triangles that fill a simple polygon, as indices into its vertices.
///
/// The painter fills a closed path only while the path is convex, and a
/// trouser front is not: the crotch is a notch. Clipping ears keeps the paper
/// inside the outline instead of spilling a sliver across it.
///
/// A polygon that cannot be clipped any further stops there, so a degenerate
/// contour loses its fill rather than the drawing losing its frame.
pub fn triangles(polygon: &[[f64; 2]]) -> Vec<[usize; 3]> {
    if polygon.len() < 3 {
        return Vec::new();
    }
    let mut rim: Vec<usize> = (0..polygon.len()).collect();
    if signed_area(polygon) < 0.0 {
        rim.reverse();
    }
    let mut out = Vec::with_capacity(rim.len() - 2);
    while rim.len() > 3 {
        let Some(at) = ear(polygon, &rim) else { break };
        let n = rim.len();
        out.push([rim[(at + n - 1) % n], rim[at], rim[(at + 1) % n]]);
        rim.remove(at);
    }
    if rim.len() == 3 {
        out.push([rim[0], rim[1], rim[2]]);
    }
    out
}

/// Where the first clippable ear sits in the remaining rim.
fn ear(polygon: &[[f64; 2]], rim: &[usize]) -> Option<usize> {
    let n = rim.len();
    (0..n).find(|&at| {
        let (i, j, k) = (rim[(at + n - 1) % n], rim[at], rim[(at + 1) % n]);
        let (a, b, c) = (polygon[i], polygon[j], polygon[k]);
        cross(a, b, c) > 0.0
            && !rim
                .iter()
                .any(|&m| m != i && m != j && m != k && inside(a, b, c, polygon[m]))
    })
}

/// Twice the signed area of the corner `a → b → c`, positive when it turns the
/// way the whole rim does.
fn cross(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

/// Whether `q` falls strictly within the triangle.
fn inside(a: [f64; 2], b: [f64; 2], c: [f64; 2], q: [f64; 2]) -> bool {
    cross(a, b, q) > 0.0 && cross(b, c, q) > 0.0 && cross(c, a, q) > 0.0
}

/// Twice the shoelace area, positive when the polygon runs counterclockwise in
/// the coordinates it is given.
fn signed_area(polygon: &[[f64; 2]]) -> f64 {
    let mut sum = 0.0;
    for (i, p) in polygon.iter().enumerate() {
        let q = polygon[(i + 1) % polygon.len()];
        sum += p[0] * q[1] - q[0] * p[1];
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The trouser front in centimetres, y downward: nine nodes with a notch
    /// at the crotch, which is the case a fan would get wrong.
    const FRONT: [[f64; 2]; 9] = [
        [0.0, 0.0],
        [22.0, 0.0],
        [25.5, 20.0],
        [21.69, 65.5],
        [20.69, 104.0],
        [-1.31, 104.0],
        [-2.31, 65.5],
        [-6.13, 27.0],
        [0.0, 20.88],
    ];

    fn covered(polygon: &[[f64; 2]], tris: &[[usize; 3]]) -> f64 {
        tris.iter()
            .map(|&[a, b, c]| cross(polygon[a], polygon[b], polygon[c]).abs() / 2.0)
            .sum()
    }

    #[test]
    fn fewer_than_three_points_fill_nothing() {
        assert!(triangles(&[]).is_empty());
        assert!(triangles(&[[0.0, 0.0], [1.0, 1.0]]).is_empty());
    }

    #[test]
    fn a_square_makes_two_triangles() {
        let square = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let tris = triangles(&square);
        assert_eq!(tris.len(), 2);
        assert!((covered(&square, &tris) - 1.0).abs() < 1.0e-9);
    }

    #[test]
    fn a_concave_piece_is_covered_exactly_once() {
        let tris = triangles(&FRONT);
        assert_eq!(tris.len(), FRONT.len() - 2);
        let area = signed_area(&FRONT).abs() / 2.0;
        assert!((covered(&FRONT, &tris) - area).abs() < 1.0e-6, "{area}");
    }

    #[test]
    fn the_winding_it_is_handed_does_not_change_the_cover() {
        let mut reversed = FRONT;
        reversed.reverse();
        let tris = triangles(&reversed);
        let area = signed_area(&reversed).abs() / 2.0;
        assert!((covered(&reversed, &tris) - area).abs() < 1.0e-6);
    }
}
