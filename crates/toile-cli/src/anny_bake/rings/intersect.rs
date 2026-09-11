use std::collections::BTreeMap;

use super::geom::{add, dot, scale, sub, unit};

/// One point where the cutting plane crosses a mesh edge: the body vertices
/// at either end of that edge, and the fraction from the first toward the
/// second. Exactly the shape [`toile_anny::asset::RingPoint`] stores, before
/// quantizing `t` to `f32`.
pub(super) type Crossing = (u32, u32, f64);

/// A vertex's signed distance from the plane through `point` with unit
/// `normal`: positive on the side `normal` points toward.
fn signed_distance(v: [f64; 3], point: [f64; 3], normal: [f64; 3]) -> f64 {
    dot(sub(v, point), normal)
}

/// Nudges the plane along its own normal, by tiny increasing multiples of a
/// fixed step, until no vertex sits exactly on it.
///
/// This mesh is built from stacked latitude rings, so thousands of vertices
/// routinely share an exact height — a plain horizontal cut at a "round"
/// height would otherwise hit that degenerate case constantly, not as a
/// rare edge case. The nudge is tiny (a tenth of a millimetre per try) and
/// deterministic, so it never visibly moves a ring; it only ever breaks a
/// tie.
///
/// # Panics
/// If no offset within 500 tries clears every vertex: the mesh would have
/// to be far stranger than this bake expects (an entire flat band of
/// vertices a full 5 cm deep) for that to happen.
fn epsilon_free_plane(positions: &[[f64; 3]], point: [f64; 3], normal: [f64; 3]) -> [f64; 3] {
    const STEP_M: f64 = 1.0e-4;
    const MAX_TRIES: u32 = 500;
    for i in 0..MAX_TRIES {
        let candidate = add(point, scale(normal, f64::from(i) * STEP_M));
        if positions
            .iter()
            .all(|&v| signed_distance(v, candidate, normal) != 0.0)
        {
            return candidate;
        }
    }
    panic!("no epsilon-free plane found near {point:?} after {MAX_TRIES} tries")
}

/// The canonical (ascending) key for the crossing on the edge between `a`
/// and `b`, so a shared edge always looks up the same [`Crossing`] whichever
/// of its two triangles asks first.
fn canonical(a: u32, b: u32) -> (u32, u32) {
    if a < b { (a, b) } else { (b, a) }
}

/// The crossing point on edge `(a, b)`, computing and caching it on first
/// request so the two triangles that share this edge agree on it exactly.
fn crossing(
    edge_point: &mut BTreeMap<(u32, u32), Crossing>,
    dist: &[f64],
    a: u32,
    b: u32,
) -> (u32, u32) {
    let key = canonical(a, b);
    edge_point.entry(key).or_insert_with(|| {
        let (da, db) = (dist[a as usize], dist[b as usize]);
        let t = da / (da - db);
        if a < b { (a, b, t) } else { (b, a, 1.0 - t) }
    });
    key
}

/// Every closed loop where the plane through `point` with `normal` crosses
/// `positions`/`tris`.
///
/// A crossed triangle contributes exactly one segment, between the two
/// crossings on its two crossed edges (the third edge does not cross,
/// because exactly one of its three vertices sits on the opposite side from
/// the other two — the plane was nudged so none sits exactly on it).
/// Segments chain through shared crossings: in a closed manifold with no
/// vertex on the plane, every crossing is shared by exactly two segments,
/// so tracing from an unused segment until it returns to its own start
/// always yields one clean cycle, and repeating from the next unused
/// segment finds every loop.
///
/// # Panics
/// If a crossing is shared by other than two segments: that would mean the
/// source mesh is not the closed, edge-manifold surface it was verified to
/// be.
pub(super) fn loops(
    positions: &[[f64; 3]],
    tris: &[[u32; 3]],
    point: [f64; 3],
    normal: [f64; 3],
) -> Vec<Vec<Crossing>> {
    let normal = unit(normal);
    let plane = epsilon_free_plane(positions, point, normal);
    let dist: Vec<f64> = positions
        .iter()
        .map(|&v| signed_distance(v, plane, normal))
        .collect();

    let mut edge_point: BTreeMap<(u32, u32), Crossing> = BTreeMap::new();
    let mut segments: Vec<((u32, u32), (u32, u32))> = Vec::new();
    for &[i, j, k] in tris {
        let d = [dist[i as usize], dist[j as usize], dist[k as usize]];
        let positive = [d[0] > 0.0, d[1] > 0.0, d[2] > 0.0];
        if positive[0] == positive[1] && positive[1] == positive[2] {
            continue;
        }
        let verts = [i, j, k];
        let lone = if positive[0] != positive[1] && positive[0] != positive[2] {
            0
        } else if positive[1] != positive[0] && positive[1] != positive[2] {
            1
        } else {
            2
        };
        let others: Vec<usize> = (0..3).filter(|&x| x != lone).collect();
        let e1 = crossing(&mut edge_point, &dist, verts[lone], verts[others[0]]);
        let e2 = crossing(&mut edge_point, &dist, verts[lone], verts[others[1]]);
        segments.push((e1, e2));
    }

    let mut node_segs: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    for (si, &(a, b)) in segments.iter().enumerate() {
        node_segs.entry(a).or_default().push(si);
        node_segs.entry(b).or_default().push(si);
    }
    for (node, segs) in &node_segs {
        assert_eq!(
            segs.len(),
            2,
            "crossing {node:?} touches {} segments, not 2",
            segs.len()
        );
    }

    let mut consumed = vec![false; segments.len()];
    let mut result: Vec<Vec<Crossing>> = Vec::new();
    for start in 0..segments.len() {
        if consumed[start] {
            continue;
        }
        let mut keys = Vec::new();
        let mut seg_idx = start;
        let (a, b) = segments[seg_idx];
        keys.push(a);
        consumed[seg_idx] = true;
        let start_node = a;
        let mut cur = b;
        while cur != start_node {
            keys.push(cur);
            let segs = &node_segs[&cur];
            let next_seg = if segs[0] == seg_idx { segs[1] } else { segs[0] };
            seg_idx = next_seg;
            consumed[seg_idx] = true;
            let (x, y) = segments[seg_idx];
            cur = if x == cur { y } else { x };
        }
        result.push(keys.into_iter().map(|k| edge_point[&k]).collect());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A single tetrahedron: one closed, manifold triangle mesh, small
    /// enough to trace by hand.
    fn tetrahedron() -> (Vec<[f64; 3]>, Vec<[u32; 3]>) {
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let tris = vec![[0, 1, 2], [0, 1, 3], [0, 2, 3], [1, 2, 3]];
        (positions, tris)
    }

    #[test]
    fn a_mid_height_plane_cuts_the_tetrahedron_into_one_triangular_loop() {
        let (positions, tris) = tetrahedron();
        let found = loops(&positions, &tris, [0.0, 0.0, 0.3], [0.0, 0.0, 1.0]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].len(), 3);
    }

    #[test]
    fn a_plane_missing_the_mesh_entirely_finds_no_loops() {
        let (positions, tris) = tetrahedron();
        let found = loops(&positions, &tris, [0.0, 0.0, 5.0], [0.0, 0.0, 1.0]);
        assert!(found.is_empty());
    }

    #[test]
    fn a_plane_through_a_shared_vertex_is_nudged_off_it() {
        // Cutting exactly at z = 0 would put vertex 0 exactly on the plane;
        // the nudge must move past it without panicking or losing the loop.
        let (positions, tris) = tetrahedron();
        let found = loops(&positions, &tris, [0.0, 0.0, 0.0], [0.0, 0.0, 1.0]);
        assert_eq!(found.len(), 1);
    }
}
