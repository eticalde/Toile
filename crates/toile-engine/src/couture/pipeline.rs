use thiserror::Error;
use toile_geom::sample;
use toile_mesh::cdt::MeshError;
use toile_mesh::{cdt, interp};
use toile_sim::xpbd::DistanceConstraints;

/// What stops an edited contour from becoming a rest state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RestStateError {
    /// A contour of a different length than the one the mesh was built from.
    ///
    /// Sampling by fraction would happily accept it and quietly corrupt the
    /// warm start, so a change of node count is refused here instead: it is a
    /// change of topology and belongs on the re-meshing path.
    #[error("the contour has {got} nodes, but the mesh was built from {expected}")]
    PointCount {
        /// What `build` saw.
        expected: usize,
        /// What `derive` was handed.
        got: usize,
    },
}

/// Where a mesh vertex sits relative to the contour.
enum VertexRole {
    /// On the contour, at this fraction of its perimeter. Includes the points
    /// spade's refinement inserted along the constrained edges.
    Boundary {
        fraction: f64,
    },
    Interior,
}

/// Compiles a contour edit into a new rest state, keeping connectivity.
///
/// Built once per piece, then every shape edit is: resample the boundary at
/// the same arc-length fractions, reproject the interior through the cached
/// interpolation matrix, recompute rest lengths. The solver only ever sees
/// changed numbers, never a changed mesh, so it warm-starts from the drape it
/// already had.
#[derive(Debug)]
pub struct ShapePipeline {
    /// Sorted arc fractions of the boundary vertices; `boundary_verts[k]` sits
    /// at `boundary_fracs[k]`.
    boundary_fracs: Vec<f64>,
    boundary_verts: Vec<u32>,
    interior_verts: Vec<u32>,
    interp: interp::BoundaryInterp,
    /// Unique edges `(a, b)`, in canonical order.
    pub edges: Vec<(u32, u32)>,
    /// Current 2D position of every vertex.
    pub pos2d: Vec<[f64; 2]>,
    /// Mesh triangles, indexing `pos2d`.
    pub tris: Vec<u32>,
    /// Rest length per edge, in the order of `edges`.
    rests: Vec<f32>,
    /// How many points the contour `build` saw had.
    contour_len: usize,
}

impl ShapePipeline {
    /// Meshes a contour and precomputes its interpolation matrix. This is the
    /// slow path, run once per piece.
    ///
    /// # Errors
    /// `MeshError` when the contour carries a coordinate the mesher cannot
    /// place.
    pub fn build(
        contour: &[[f64; 2]],
        n_samples: usize,
        max_area: f64,
    ) -> Result<ShapePipeline, MeshError> {
        let fractions = sample::uniform_fractions(n_samples);
        let boundary = sample::sample_closed(contour, &fractions);
        let mesh = cdt::triangulate(&boundary, max_area)?;

        let roles: Vec<VertexRole> = mesh
            .vertices
            .iter()
            .map(|v| classify(*v, &boundary, &fractions))
            .collect();

        let mut boundary_pairs: Vec<(f64, u32)> = Vec::new();
        let mut interior_verts: Vec<u32> = Vec::new();
        for (i, r) in roles.iter().enumerate() {
            match r {
                VertexRole::Boundary { fraction } => boundary_pairs.push((*fraction, i as u32)),
                VertexRole::Interior => interior_verts.push(i as u32),
            }
        }
        boundary_pairs.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let boundary_fracs: Vec<f64> = boundary_pairs.iter().map(|p| p.0).collect();
        let boundary_verts: Vec<u32> = boundary_pairs.iter().map(|p| p.1).collect();

        let boundary_poly: Vec<[f64; 2]> = boundary_verts
            .iter()
            .map(|&v| mesh.vertices[v as usize])
            .collect();
        let interior_pts: Vec<[f64; 2]> = interior_verts
            .iter()
            .map(|&v| mesh.vertices[v as usize])
            .collect();
        let interp = interp::mvc_weights(&boundary_poly, &interior_pts);

        let mut edges: Vec<(u32, u32)> = Vec::with_capacity(mesh.triangles.len());
        for t in mesh.triangles.chunks(3) {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                edges.push((a.min(b), a.max(b)));
            }
        }
        edges.sort_unstable();
        edges.dedup();

        let mut me = Self {
            boundary_fracs,
            boundary_verts,
            interior_verts,
            interp,
            edges,
            tris: mesh.triangles,
            pos2d: mesh.vertices,
            rests: Vec::new(),
            contour_len: contour.len(),
        };
        me.rests = vec![0.0; me.edges.len()];
        me.recompute_rests();
        Ok(me)
    }

    /// Recompiles an edited contour into new rest lengths, in `edges` order.
    ///
    /// # Errors
    /// `RestStateError::PointCount` when the contour has gained or lost a
    /// node since `build`. That is a topology edit wearing a shape edit's
    /// clothes, and taking it would corrupt the warm start in silence.
    pub fn derive(&mut self, contour: &[[f64; 2]]) -> Result<&[f32], RestStateError> {
        if contour.len() != self.contour_len {
            return Err(RestStateError::PointCount {
                expected: self.contour_len,
                got: contour.len(),
            });
        }
        let boundary_new = sample::sample_closed(contour, &self.boundary_fracs);
        for (k, &v) in self.boundary_verts.iter().enumerate() {
            self.pos2d[v as usize] = boundary_new[k];
        }
        let mut interior_new = vec![[0.0f64; 2]; self.interior_verts.len()];
        interp::apply(&self.interp, &boundary_new, &mut interior_new);
        for (k, &v) in self.interior_verts.iter().enumerate() {
            self.pos2d[v as usize] = interior_new[k];
        }
        self.recompute_rests();
        Ok(&self.rests)
    }

    /// How many points the contour this mesh was built from had.
    pub fn contour_len(&self) -> usize {
        self.contour_len
    }

    fn recompute_rests(&mut self) {
        for (k, &(a, b)) in self.edges.iter().enumerate() {
            let (pa, pb) = (self.pos2d[a as usize], self.pos2d[b as usize]);
            self.rests[k] = (((pb[0] - pa[0]).powi(2) + (pb[1] - pa[1]).powi(2)).sqrt()) as f32;
        }
    }

    /// Seeds the solver with uniform compliance. Warp/weft anisotropy replaces
    /// the uniform value once fabric presets exist.
    pub fn constraints(&self, compliance: f32) -> DistanceConstraints {
        DistanceConstraints {
            a: self.edges.iter().map(|e| e.0).collect(),
            b: self.edges.iter().map(|e| e.1).collect(),
            rest: self.rests.clone(),
            compliance: vec![compliance; self.edges.len()],
            strain_limit: 0.0,
            strain_sweeps: 0,
        }
    }

    /// Number of boundary vertices.
    pub fn n_boundary(&self) -> usize {
        self.boundary_verts.len()
    }

    /// Number of interior vertices.
    pub fn n_interior(&self) -> usize {
        self.interior_verts.len()
    }

    /// The boundary vertex nearest a fraction of the perimeter.
    pub fn boundary_vertex_near(&self, fraction: f64) -> u32 {
        let fr = fraction.rem_euclid(1.0);
        let n = self.boundary_fracs.len();
        let i = self.boundary_fracs.partition_point(|&x| x < fr);
        let circ = |d: f64| d.abs().min(1.0 - d.abs());
        let prev = (i + n - 1) % n;
        let next = i % n;
        if circ(self.boundary_fracs[prev] - fr) <= circ(self.boundary_fracs[next] - fr) {
            self.boundary_verts[prev]
        } else {
            self.boundary_verts[next]
        }
    }
}

/// Classifies a mesh vertex by distance to the sampled boundary polygon,
/// recovering its arc fraction when it lies on it.
fn classify(p: [f64; 2], boundary: &[[f64; 2]], fractions: &[f64]) -> VertexRole {
    const EPS: f64 = 1.0e-9;
    let n = boundary.len();
    for i in 0..n {
        let (a, b) = (boundary[i], boundary[(i + 1) % n]);
        let (ex, ey) = (b[0] - a[0], b[1] - a[1]);
        let len2 = ex * ex + ey * ey;
        let t = if len2 > 0.0 {
            (((p[0] - a[0]) * ex + (p[1] - a[1]) * ey) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let (cx, cy) = (a[0] + ex * t - p[0], a[1] + ey * t - p[1]);
        if cx * cx + cy * cy < EPS * EPS {
            let next = if i + 1 < n { fractions[i + 1] } else { 1.0 };
            return VertexRole::Boundary {
                fraction: fractions[i] + (next - fractions[i]) * t,
            };
        }
    }
    VertexRole::Interior
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A rectangle, corners only: cheap to mesh and easy to edit.
    fn rectangle() -> Vec<[f64; 2]> {
        vec![[0.0, 0.0], [0.30, 0.0], [0.30, 0.20], [0.0, 0.20]]
    }

    fn pipeline() -> ShapePipeline {
        ShapePipeline::build(&rectangle(), 16, 0.01).expect("the rectangle is finite")
    }

    #[test]
    fn derive_with_a_different_point_count_is_an_error() {
        let mut pipe = pipeline();
        let mut grown = rectangle();
        grown.push([0.15, 0.30]);
        assert_eq!(
            pipe.derive(&grown),
            Err(RestStateError::PointCount {
                expected: 4,
                got: 5
            })
        );
    }

    #[test]
    fn a_moved_node_keeps_the_mesh_and_changes_the_rest_lengths() {
        let mut pipe = pipeline();
        let before = pipe.rests.clone();
        let mut edited = rectangle();
        edited[1][0] += 0.05;
        let after = pipe.derive(&edited).expect("the node count did not move");
        assert_eq!(after.len(), before.len());
        assert_ne!(after, before.as_slice());
        assert_eq!(pipe.contour_len(), 4);
    }

    #[test]
    fn a_contour_the_mesher_refuses_is_an_error_not_a_panic() {
        let mut broken = rectangle();
        broken[2][0] = f64::NAN;
        assert_eq!(
            ShapePipeline::build(&broken, 16, 0.01).err(),
            Some(MeshError::NonFiniteVertex { index: 0 })
        );
    }
}
