/// The distance from a point to the nearest triangle of a mesh, the slow way.
///
/// Every triangle, no band, no bounding box — and, deliberately, not the
/// Voronoi-region case analysis the bake uses. This drops a perpendicular
/// and asks whether its foot is inside the triangle by the sign of three
/// cross products, falling back to the three edges when it is not. Two
/// routes to the same number is the whole point: agreement means the answer
/// is the geometry rather than one routine's idea of it.
pub(super) fn distance(positions: &[f32], indices: &[u32], q: [f64; 3]) -> f64 {
    indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| to_triangle(q, [0, 1, 2].map(|k| at(positions, t[k]))))
        .fold(f64::INFINITY, f64::min)
}

/// How many times a mesh's surface wraps a point: one inside a closed
/// surface, zero outside it.
///
/// The solid angle every triangle covers, summed. It asks how often the skin
/// goes round the point rather than which piece of skin is nearest, so it can
/// be asked whether the sign the bake read off the nearest sheet was right.
pub(super) fn winding(positions: &[f32], indices: &[u32], q: [f64; 3]) -> f64 {
    let turns: f64 = indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| {
            let [a, b, c] = [0, 1, 2].map(|k| sub(at(positions, t[k]), q));
            let (la, lb, lc) = (length(a), length(b), length(c));
            let over = dot(a, cross(b, c));
            let under = la * lb * lc + dot(a, b) * lc + dot(a, c) * lb + dot(b, c) * la;
            2.0 * over.atan2(under)
        })
        .sum();
    turns / (4.0 * std::f64::consts::PI)
}

/// How many pairs of `tris` pass through each other.
///
/// Pairs sharing a vertex are skipped: two triangles of one fan always meet
/// along the edge they share, and that is the surface being a surface rather
/// than it crossing itself. What is left is the condition no half-edge count
/// can see — two sheets of one skin in the same place.
pub(super) fn crossings(positions: &[f32], tris: &[[u32; 3]]) -> usize {
    let mut found = 0;
    for (n, a) in tris.iter().enumerate() {
        for b in &tris[n + 1..] {
            if a.iter().any(|v| b.contains(v)) {
                continue;
            }
            let p = [0, 1, 2].map(|k| at(positions, a[k]));
            let q = [0, 1, 2].map(|k| at(positions, b[k]));
            found += usize::from(crosses(p, q));
        }
    }
    found
}

/// Whether two triangles sharing no vertex pass through each other.
///
/// Each is cut by the other's plane, and both cuts land on the one line the
/// two planes share. The triangles meet exactly where those two cuts overlap.
fn crosses(p: [[f64; 3]; 3], q: [[f64; 3]; 3]) -> bool {
    let plane = |t: [[f64; 3]; 3]| {
        let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
        (n, -dot(n, t[0]))
    };
    let (n2, d2) = plane(q);
    let (n1, d1) = plane(p);
    let dp = [0, 1, 2].map(|i| dot(n2, p[i]) + d2);
    let dq = [0, 1, 2].map(|i| dot(n1, q[i]) + d1);
    let apart = |d: [f64; 3]| d.iter().all(|&x| x > 0.0) || d.iter().all(|&x| x < 0.0);
    if apart(dp) || apart(dq) {
        return false;
    }
    let line = cross(n1, n2);
    let axis = (0..3).fold(0, |best, c| {
        if line[c].abs() > line[best].abs() {
            c
        } else {
            best
        }
    });
    match (cut(p.map(|v| v[axis]), dp), cut(q.map(|v| v[axis]), dq)) {
        (Some(a), Some(b)) => a.0.max(b.0) < a.1.min(b.1),
        _ => false,
    }
}

/// Where a triangle's cut by the other plane starts and ends, measured along
/// the axis the two planes' shared line runs most steeply.
fn cut(v: [f64; 3], d: [f64; 3]) -> Option<(f64, f64)> {
    let mut ends: Vec<f64> = Vec::new();
    for i in 0..3 {
        let j = (i + 1) % 3;
        if d[i] == 0.0 {
            ends.push(v[i]);
        }
        if d[i] * d[j] < 0.0 {
            ends.push(v[i] + (v[j] - v[i]) * d[i] / (d[i] - d[j]));
        }
    }
    let lo = ends.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = ends.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (ends.len() >= 2).then_some((lo, hi))
}

/// One vertex as a `f64` point.
fn at(positions: &[f32], i: u32) -> [f64; 3] {
    let i = i as usize * 3;
    [0, 1, 2].map(|k| f64::from(positions[i + k]))
}

/// The distance from `q` to one triangle.
fn to_triangle(q: [f64; 3], p: [[f64; 3]; 3]) -> f64 {
    let n = cross(sub(p[1], p[0]), sub(p[2], p[0]));
    let area2 = dot(n, n);
    if area2 > 0.0 {
        let along = dot(n, sub(q, p[0])) / area2;
        let foot = [0, 1, 2].map(|c| q[c] - n[c] * along);
        let inside = (0..3).all(|k| {
            let edge = sub(p[(k + 1) % 3], p[k]);
            dot(n, cross(edge, sub(foot, p[k]))) >= 0.0
        });
        if inside {
            return length(sub(q, foot));
        }
    }
    (0..3)
        .map(|k| to_segment(q, p[k], p[(k + 1) % 3]))
        .fold(f64::INFINITY, f64::min)
}

/// The distance from `q` to the segment `a`–`b`.
fn to_segment(q: [f64; 3], a: [f64; 3], b: [f64; 3]) -> f64 {
    let ab = sub(b, a);
    let span = dot(ab, ab);
    let t = if span > 0.0 {
        (dot(sub(q, a), ab) / span).clamp(0.0, 1.0)
    } else {
        0.0
    };
    length(sub(q, [0, 1, 2].map(|c| a[c] + ab[c] * t)))
}

/// `a − b`, component by component.
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|c| a[c] - b[c])
}

/// The dot product of two vectors.
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The cross product of two vectors.
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// A vector's length.
fn length(v: [f64; 3]) -> f64 {
    dot(v, v).sqrt()
}
