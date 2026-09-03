use toile_geom::sample;
use toile_mesh::{cdt, interp};
use toile_sim::xpbd::DistanceConstraints;

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
}

impl ShapePipeline {
    /// Meshes a contour and precomputes its interpolation matrix. This is the
    /// slow path, run once per piece.
    pub fn build(contour: &[[f64; 2]], n_samples: usize, max_area: f64) -> Self {
        let fractions = sample::uniform_fractions(n_samples);
        let boundary = sample::sample_closed(contour, &fractions);
        let mesh = cdt::triangulate(&boundary, max_area);

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
        };
        me.rests = vec![0.0; me.edges.len()];
        me.recompute_rests();
        me
    }

    /// Recompiles an edited contour into new rest lengths, in `edges` order.
    pub fn derive(&mut self, contour: &[[f64; 2]]) -> &[f32] {
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
        &self.rests
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
