use super::Lattice;
use super::pseudo::{Feature, Pseudo};

/// Writes the true signed distance into every sample within the band of some
/// triangle, walking the triangles in index order.
///
/// `nearest` carries each sample's squared distance to the closest triangle
/// found so far, and is what makes the pass order-independent in its result:
/// a later triangle only overwrites a sample when it is strictly nearer, so
/// a sample equidistant from two triangles takes the sign of the lower
/// index. That tie has to be broken by something fixed, and the index is the
/// only thing at hand that does not move with the arithmetic.
pub(super) fn walk(
    lattice: &Lattice,
    positions: &[f32],
    indices: &[u32],
    pseudo: &Pseudo,
    data: &mut [f32],
    nearest: &mut [f32],
) {
    let band = lattice.band;
    let reach = band * band;
    for (t, corners) in indices.as_chunks::<3>().0.iter().enumerate() {
        let p = [0, 1, 2].map(|k| at(positions, corners[k]));
        let axes = [0, 1, 2].map(|c| {
            let lo = p[0][c].min(p[1][c]).min(p[2][c]) - band;
            let hi = p[0][c].max(p[1][c]).max(p[2][c]) + band;
            lattice.span(c, lo, hi)
        });
        for k in axes[2].clone() {
            for j in axes[1].clone() {
                for i in axes[0].clone() {
                    let q = lattice.point(i, j, k);
                    let (x, feature) = closest(p, q);
                    let away = [0, 1, 2].map(|c| q[c] - x[c]);
                    let square = away[0] * away[0] + away[1] * away[1] + away[2] * away[2];
                    if square > reach {
                        continue;
                    }
                    let slot = lattice.index(i, j, k);
                    if (square as f32) < nearest[slot] {
                        nearest[slot] = square as f32;
                        let n = pseudo.normal(t, *corners, feature);
                        let outward = n[0] * away[0] + n[1] * away[1] + n[2] * away[2];
                        let d = square.sqrt();
                        data[slot] = if outward < 0.0 { -d as f32 } else { d as f32 };
                    }
                }
            }
        }
    }
}

/// One vertex as a `f64` point.
fn at(positions: &[f32], i: u32) -> [f64; 3] {
    let i = i as usize * 3;
    [0, 1, 2].map(|k| f64::from(positions[i + k]))
}

/// The point of a triangle closest to `q`, and which part of the triangle it
/// landed on.
///
/// The Voronoi-region case analysis: each test below asks which side of one
/// region boundary `q` lies on, so the answer is reached in `+ − × ÷` and
/// one `sqrt` at the caller. The feature is not a by-product — it is half
/// the answer, because the sign comes from that feature's pseudo-normal and
/// from no other.
fn closest(p: [[f64; 3]; 3], q: [f64; 3]) -> ([f64; 3], Feature) {
    let ab = sub(p[1], p[0]);
    let ac = sub(p[2], p[0]);
    let aq = sub(q, p[0]);
    let d1 = dot(ab, aq);
    let d2 = dot(ac, aq);
    if d1 <= 0.0 && d2 <= 0.0 {
        return (p[0], Feature::Vertex(0));
    }
    let bq = sub(q, p[1]);
    let d3 = dot(ab, bq);
    let d4 = dot(ac, bq);
    if d3 >= 0.0 && d4 <= d3 {
        return (p[1], Feature::Vertex(1));
    }
    let cq = sub(q, p[2]);
    let d5 = dot(ab, cq);
    let d6 = dot(ac, cq);
    if d6 >= 0.0 && d5 <= d6 {
        return (p[2], Feature::Vertex(2));
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return (along(p[0], ab, d1 / (d1 - d3)), Feature::Edge(0));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        let t = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (along(p[1], sub(p[2], p[1]), t), Feature::Edge(1));
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return (along(p[0], ac, d2 / (d2 - d6)), Feature::Edge(2));
    }
    let scale = 1.0 / (va + vb + vc);
    let v = vb * scale;
    let w = vc * scale;
    (
        [0, 1, 2].map(|c| p[0][c] + ab[c] * v + ac[c] * w),
        Feature::Face,
    )
}

/// `a − b`, component by component.
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|c| a[c] - b[c])
}

/// The dot product of two vectors.
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The point `t` of the way along `d` from `from`.
fn along(from: [f64; 3], d: [f64; 3], t: f64) -> [f64; 3] {
    [0, 1, 2].map(|c| from[c] + d[c] * t)
}
