use super::state::State;
use super::vector::{cross, dot, sub};

/// Below this the edge runs along the triangle's plane and pierces nothing.
const PARALLEL: f32 = 1.0e-20;

/// Edges of the mesh that pass clean through a triangle they share no vertex
/// with: cloth that has gone through itself.
///
/// Deliberately not the proximity [`super::Layers`] acts on. A pass that is
/// judged by its own test grades its own paper; this counts a crossing that
/// has already happened, which is the thing a person sees.
///
/// Sharing a vertex is the whole exclusion, and it needs no adjacency: two
/// triangles that meet at a corner or an edge cannot cross each other
/// transversally, and a garment's own seam allowance lies against itself
/// without ever piercing.
///
/// Brute force with a box reject in front. It is a measurement, it runs where
/// a test asks for it, and it never runs on the sim thread.
pub fn self_crossings(state: &State, tris: &[u32]) -> usize {
    let n = state.len();
    let edges = edges_of(tris, n);
    let mut found = 0;
    for t in tris.as_chunks::<3>().0 {
        if t.iter().any(|&v| v as usize >= n) {
            continue;
        }
        let corner = [at(state, t[0]), at(state, t[1]), at(state, t[2])];
        let (lo, hi) = box_of(corner);
        for &(a, b) in &edges {
            if t.contains(&a) || t.contains(&b) {
                continue;
            }
            let (p, q) = (at(state, a), at(state, b));
            if misses(p, q, lo, hi) {
                continue;
            }
            if pierces(p, q, corner) {
                found += 1;
            }
        }
    }
    found
}

/// Every undirected edge of the mesh, once, in canonical order.
fn edges_of(tris: &[u32], n_verts: usize) -> Vec<(u32, u32)> {
    let mut edges = Vec::with_capacity(tris.len());
    for t in tris.as_chunks::<3>().0 {
        if t.iter().any(|&v| v as usize >= n_verts) {
            continue;
        }
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            edges.push((a.min(b), a.max(b)));
        }
    }
    edges.sort_unstable();
    edges.dedup();
    edges
}

/// Moller-Trumbore, with the segment's own parameter kept inside its ends.
fn pierces(p: [f32; 3], q: [f32; 3], t: [[f32; 3]; 3]) -> bool {
    let dir = sub(q, p);
    let e1 = sub(t[1], t[0]);
    let e2 = sub(t[2], t[0]);
    let across = cross(dir, e2);
    let det = dot(e1, across);
    if det > -PARALLEL && det < PARALLEL {
        return false;
    }
    let inv = 1.0 / det;
    let from = sub(p, t[0]);
    let u = dot(from, across) * inv;
    if !(0.0..=1.0).contains(&u) {
        return false;
    }
    let along = cross(from, e1);
    let v = dot(dir, along) * inv;
    if v < 0.0 || u + v > 1.0 {
        return false;
    }
    let reach = dot(e2, along) * inv;
    reach > 0.0 && reach < 1.0
}

/// Whether a segment's own box misses the triangle's altogether.
fn misses(p: [f32; 3], q: [f32; 3], lo: [f32; 3], hi: [f32; 3]) -> bool {
    (0..3).any(|c| p[c].min(q[c]) > hi[c] || p[c].max(q[c]) < lo[c])
}

fn box_of(t: [[f32; 3]; 3]) -> ([f32; 3], [f32; 3]) {
    let mut lo = [f32::INFINITY; 3];
    let mut hi = [f32::NEG_INFINITY; 3];
    for corner in t {
        for c in 0..3 {
            lo[c] = lo[c].min(corner[c]);
            hi[c] = hi[c].max(corner[c]);
        }
    }
    (lo, hi)
}

fn at(state: &State, i: u32) -> [f32; 3] {
    let i = i as usize;
    [state.px[i], state.py[i], state.pz[i]]
}
