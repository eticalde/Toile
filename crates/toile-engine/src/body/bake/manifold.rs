/// The two vertices of an edge as one sortable key, smaller index first, so
/// the two triangles that share an edge agree on its name whichever way each
/// of them runs it.
pub(super) fn edge_key(a: u32, b: u32) -> u64 {
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    (u64::from(lo) << 32) | u64::from(hi)
}

/// What a mesh's topology is, in the terms the pseudo-normal sign needs.
///
/// The sign of a distance is `dot(pseudo_normal, p − x)`, and that is only
/// provably the true inside/outside answer for a closed, orientable
/// 2-manifold surface that does not pass through itself. A mesh with a hole,
/// a T-junction or a flipped face gets a field that is right almost
/// everywhere and wrong in a region no test of the solver would point at, so
/// the mesh is checked before it is baked rather than after something drapes
/// oddly.
///
/// Everything counted below is combinatorial, and self-intersection is not:
/// two sheets pass through each other without any edge count noticing. Where
/// they do, the nearest sheet to a point need not be the sheet that bounds
/// it, and the sign follows the nearest. Catching that means the triangles
/// against each other rather than the half-edges against themselves, which
/// is a different pass and a far costlier one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Manifold {
    /// Vertices in the mesh.
    pub vertices: usize,
    /// Triangles in the mesh.
    pub triangles: usize,
    /// Distinct undirected edges.
    pub edges: usize,
    /// Edges used by exactly one triangle: the rim of a hole.
    pub boundary_edges: usize,
    /// Edges used by three or more triangles.
    pub nonmanifold_edges: usize,
    /// Edges whose two triangles run them the same way round instead of in
    /// opposite directions: one of the two faces is flipped.
    pub inconsistent_edges: usize,
    /// Triangles with a repeated vertex or no area, which have no normal.
    pub degenerate_triangles: usize,
    /// `V − E + F`. A single closed surface of genus 0 gives 2.
    pub euler: isize,
    /// The volume the surface encloses, in litres, summed one tetrahedron
    /// per triangle from the origin. Positive exactly when the winding is
    /// counter-clockwise seen from outside, which is the direction the face
    /// normals inherit.
    pub litres: f64,
}

impl Manifold {
    /// Whether this mesh earns the sign: closed, 2-manifold, consistently
    /// wound counter-clockwise outward, with no face that has no normal.
    ///
    /// Embedded is the condition it cannot answer for, and Anny's soles fail
    /// it. At the default phenotype 52 pairs of sole triangles pass through
    /// each other, and the field baked from that mesh reads four voxels of
    /// solid sole as air: where two sheets cross, the nearest one is not the
    /// one that bounds the point, and the sign follows the nearest. The body
    /// the goldens are taken against crosses no pair and loses no voxel,
    /// which is why the pinned field is clean while this one is not. All four
    /// are buried inside a sole, where the solver never holds a particle.
    pub fn earns_the_sign(&self) -> bool {
        self.boundary_edges == 0
            && self.nonmanifold_edges == 0
            && self.inconsistent_edges == 0
            && self.degenerate_triangles == 0
            && self.litres > 0.0
    }
}

/// Counts everything [`Manifold`] reports, in one pass over the half-edges
/// and one over the triangles.
///
/// Every directed edge of every triangle becomes a `(key, forward)` pair;
/// sorting brings the pairs that name the same undirected edge together, and
/// a group of exactly two with exactly one `forward` is the healthy case.
pub fn inspect(positions: &[f32], indices: &[u32]) -> Manifold {
    let tris = indices.as_chunks::<3>().0;
    let mut halves: Vec<(u64, bool)> = Vec::with_capacity(tris.len() * 3);
    for t in tris {
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            halves.push((edge_key(a, b), a < b));
        }
    }
    halves.sort_unstable();

    let (mut edges, mut boundary, mut nonmanifold, mut inconsistent) = (0, 0, 0, 0);
    let mut i = 0;
    while i < halves.len() {
        let mut j = i;
        let mut forward = 0;
        while j < halves.len() && halves[j].0 == halves[i].0 {
            forward += usize::from(halves[j].1);
            j += 1;
        }
        edges += 1;
        match j - i {
            1 => boundary += 1,
            2 => inconsistent += usize::from(forward != 1),
            _ => nonmanifold += 1,
        }
        i = j;
    }

    let vertices = positions.len() / 3;
    let (degenerate, volume_x6) = areas_and_volume(positions, tris);
    Manifold {
        vertices,
        triangles: tris.len(),
        edges,
        boundary_edges: boundary,
        nonmanifold_edges: nonmanifold,
        inconsistent_edges: inconsistent,
        degenerate_triangles: degenerate,
        euler: vertices as isize - edges as isize + tris.len() as isize,
        litres: volume_x6 / 6.0 * 1000.0,
    }
}

/// How many triangles have no normal, and six times the signed volume.
///
/// The two travel together because both are a cross product per triangle:
/// the same one, read twice.
fn areas_and_volume(positions: &[f32], tris: &[[u32; 3]]) -> (usize, f64) {
    let at = |i: u32| {
        let i = i as usize * 3;
        [0, 1, 2].map(|k| f64::from(positions[i + k]))
    };
    let mut degenerate = 0;
    let mut volume_x6 = 0.0;
    for t in tris {
        let (a, b, c) = (at(t[0]), at(t[1]), at(t[2]));
        let e1 = [0, 1, 2].map(|k| b[k] - a[k]);
        let e2 = [0, 1, 2].map(|k| c[k] - a[k]);
        let n = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        if n[0] * n[0] + n[1] * n[1] + n[2] * n[2] == 0.0 {
            degenerate += 1;
        }
        let cross = [
            b[1] * c[2] - b[2] * c[1],
            b[2] * c[0] - b[0] * c[2],
            b[0] * c[1] - b[1] * c[0],
        ];
        volume_x6 += a[0] * cross[0] + a[1] * cross[1] + a[2] * cross[2];
    }
    (degenerate, volume_x6)
}
