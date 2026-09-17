use super::manifold::edge_key;

/// Which part of a triangle a point's closest point landed on.
///
/// `Edge(k)` is the edge from corner `k` to corner `k + 1`, and `Vertex(k)`
/// is corner `k`, so a feature plus a triangle names one pseudo-normal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Feature {
    /// The interior of the triangle.
    Face,
    /// The edge leaving corner `k`.
    Edge(usize),
    /// Corner `k`.
    Vertex(usize),
}

/// The pseudo-normals of Bærentzen–Aanæs: one per face, one per edge, one
/// per vertex.
///
/// A face's is its unit normal, an edge's is the sum of its two faces', and
/// a vertex's is the sum of its faces' weighted by the angle each subtends
/// at that vertex. Those weights are what make the sign right at a vertex,
/// and they are the reason `libm` is a dependency here: the angle is an
/// `acos`.
///
/// None of the three is normalized. Only the sign of a dot product is ever
/// read off them, and a positive scale cannot change a sign.
pub(super) struct Pseudo {
    face: Vec<[f64; 3]>,
    vertex: Vec<[f64; 3]>,
    edge_keys: Vec<u64>,
    edge_normals: Vec<[f64; 3]>,
}

impl Pseudo {
    /// Builds all three tables in one walk of the triangles, in index order.
    ///
    /// The order is the whole determinism story: every sum below is a sum of
    /// floats, so it is the fixed order of the terms, not the values, that
    /// makes the result the same on ARM and x86.
    pub(super) fn build(positions: &[f32], indices: &[u32]) -> Self {
        let tris = indices.as_chunks::<3>().0;
        let mut edge_keys: Vec<u64> = tris
            .iter()
            .flat_map(|t| (0..3).map(|k| edge_key(t[k], t[(k + 1) % 3])))
            .collect();
        edge_keys.sort_unstable();
        edge_keys.dedup();

        let mut me = Pseudo {
            face: Vec::with_capacity(tris.len()),
            vertex: vec![[0.0; 3]; positions.len() / 3],
            edge_normals: vec![[0.0; 3]; edge_keys.len()],
            edge_keys,
        };
        for t in tris {
            let corners = [0, 1, 2].map(|k| at(positions, t[k]));
            let n = unit_normal(corners);
            me.face.push(n);
            for k in 0..3 {
                let slot = me.edge_slot(edge_key(t[k], t[(k + 1) % 3]));
                accumulate(&mut me.edge_normals[slot], n, 1.0);
                accumulate(&mut me.vertex[t[k] as usize], n, corner_angle(corners, k));
            }
        }
        me
    }

    /// The pseudo-normal of one feature of one triangle.
    pub(super) fn normal(&self, tri: usize, corners: [u32; 3], feature: Feature) -> [f64; 3] {
        match feature {
            Feature::Face => self.face[tri],
            Feature::Edge(k) => {
                self.edge_normals[self.edge_slot(edge_key(corners[k], corners[(k + 1) % 3]))]
            }
            Feature::Vertex(k) => self.vertex[corners[k] as usize],
        }
    }

    /// Where an edge's accumulated normal lives.
    ///
    /// # Panics
    /// If the key is not one of the mesh's own edges, which cannot happen:
    /// every key asked for is built from a triangle of the same mesh the
    /// table was built from.
    fn edge_slot(&self, key: u64) -> usize {
        self.edge_keys
            .binary_search(&key)
            .expect("every edge asked for came from a triangle of this mesh")
    }
}

/// Adds `weight * v` into a running pseudo-normal.
fn accumulate(target: &mut [f64; 3], v: [f64; 3], weight: f64) {
    for (t, c) in target.iter_mut().zip(v) {
        *t += weight * c;
    }
}

/// One vertex as a `f64` point.
fn at(positions: &[f32], i: u32) -> [f64; 3] {
    let i = i as usize * 3;
    [0, 1, 2].map(|k| f64::from(positions[i + k]))
}

/// A triangle's unit normal, counter-clockwise outward for this mesh's
/// winding.
///
/// A triangle with no area would divide by zero here; the caller refuses
/// such a mesh before building this table (see [`super::manifold`]).
fn unit_normal(p: [[f64; 3]; 3]) -> [f64; 3] {
    let e1 = [0, 1, 2].map(|k| p[1][k] - p[0][k]);
    let e2 = [0, 1, 2].map(|k| p[2][k] - p[0][k]);
    let n = [
        e1[1] * e2[2] - e1[2] * e2[1],
        e1[2] * e2[0] - e1[0] * e2[2],
        e1[0] * e2[1] - e1[1] * e2[0],
    ];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    n.map(|c| c / len)
}

/// The interior angle a triangle subtends at corner `k`, in radians.
///
/// The cosine is clamped before the `acos` because a very thin triangle can
/// compute a ratio a hair outside `[-1, 1]` in floating point, and `acos`
/// answers that with a NaN that would poison a vertex's whole sum.
fn corner_angle(p: [[f64; 3]; 3], k: usize) -> f64 {
    let u = [0, 1, 2].map(|c| p[(k + 1) % 3][c] - p[k][c]);
    let v = [0, 1, 2].map(|c| p[(k + 2) % 3][c] - p[k][c]);
    let dot = u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
    let lu = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt();
    let lv = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    libm::acos((dot / (lu * lv)).clamp(-1.0, 1.0))
}
