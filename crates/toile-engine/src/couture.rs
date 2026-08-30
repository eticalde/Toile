//! Módulo *couture* — el compilador de ediciones (ADR §2.7).
//!
//! Spike 2: la vía A (cambio de forma). Construye una vez la malla y la
//! matriz de interpolación; después cada edición del contorno se compila a
//! un nuevo estado de reposo con conectividad intacta:
//!
//! contorno editado → re-muestreo por fracciones → interior por MVC →
//! rest lengths nuevos → hot-swap en el solver residente.

use toile_geom::sample;
use toile_mesh::{cdt, interp};
use toile_sim::xpbd::DistanceConstraints;

/// Rol de cada vértice de la malla respecto al contorno.
enum VertexRole {
    /// En el contorno, en esta fracción de su perímetro. Incluye los puntos
    /// que el refinement de spade insertó sobre los bordes restringidos.
    Boundary { fraction: f64 },
    /// Interior: interpolado desde la frontera vía MVC.
    Interior,
}

pub struct ShapePipeline {
    /// Fracciones de arco de los vértices de frontera, ordenadas; el
    /// vértice `boundary_verts[k]` vive en `boundary_fracs[k]`.
    boundary_fracs: Vec<f64>,
    boundary_verts: Vec<u32>,
    interior_verts: Vec<u32>,
    interp: interp::BoundaryInterp,
    /// Aristas únicas (a, b) en orden canónico.
    pub edges: Vec<(u32, u32)>,
    /// Posiciones 2D actuales de todos los vértices (se actualizan en derive).
    pub pos2d: Vec<[f64; 2]>,
    /// Buffer de salida: rest length por arista, en el orden de `edges`.
    rests: Vec<f32>,
}

impl ShapePipeline {
    /// Construye malla + matriz para un contorno (una vez, ruta lenta).
    pub fn build(contour: &[[f64; 2]], n_samples: usize, max_area: f64) -> Self {
        let fractions = sample::uniform_fractions(n_samples);
        let boundary = sample::sample_closed(contour, &fractions);
        let mesh = cdt::triangulate(&boundary, max_area);

        // Clasificación de vértices contra el polígono muestreado.
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

        // Aristas únicas de los triángulos, orden canónico.
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
            pos2d: mesh.vertices,
            rests: Vec::new(),
        };
        me.rests = vec![0.0; me.edges.len()];
        me.recompute_rests();
        me
    }

    /// Vía A completa para un contorno editado: re-muestrea la frontera en
    /// las MISMAS fracciones, reposiciona el interior vía MVC y recalcula
    /// los rest lengths. Conectividad intacta — el solver solo cambia
    /// números. Devuelve los rests en el orden de `edges`.
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

    /// Constraints iniciales para sembrar el solver (compliance uniforme
    /// de spike; la anisotropía llega con el solver real).
    pub fn constraints(&self, compliance: f32) -> DistanceConstraints {
        DistanceConstraints {
            a: self.edges.iter().map(|e| e.0).collect(),
            b: self.edges.iter().map(|e| e.1).collect(),
            rest: self.rests.clone(),
            compliance: vec![compliance; self.edges.len()],
            strain_limit: 0.0,
        }
    }

    pub fn n_boundary(&self) -> usize {
        self.boundary_verts.len()
    }

    pub fn n_interior(&self) -> usize {
        self.interior_verts.len()
    }
}

/// Distancia del vértice al polígono muestreado; si está sobre él (los
/// vértices originales y los splits del refinement), su fracción de arco.
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

/// Contorno de demostración: delantero de corpiño con sisa y escote
/// cóncavos. Fixture compartido por benches, goldens y demos.
pub fn demo_bodice_contour() -> Vec<[f64; 2]> {
    let mut pts: Vec<[f64; 2]> = Vec::new();
    let line = |pts: &mut Vec<[f64; 2]>, a: [f64; 2], b: [f64; 2], n: usize| {
        for i in 0..n {
            let t = i as f64 / n as f64;
            pts.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
        }
    };
    let quad = |pts: &mut Vec<[f64; 2]>, a: [f64; 2], c: [f64; 2], b: [f64; 2], n: usize| {
        for i in 0..n {
            let t = i as f64 / n as f64;
            let u = 1.0 - t;
            pts.push([
                u * u * a[0] + 2.0 * u * t * c[0] + t * t * b[0],
                u * u * a[1] + 2.0 * u * t * c[1] + t * t * b[1],
            ]);
        }
    };
    line(&mut pts, [0.0, 0.0], [0.50, 0.0], 20); // cintura
    line(&mut pts, [0.50, 0.0], [0.52, 0.45], 18); // costado
    quad(&mut pts, [0.52, 0.45], [0.36, 0.50], [0.38, 0.68], 30); // sisa
    line(&mut pts, [0.38, 0.68], [0.18, 0.72], 10); // hombro
    quad(&mut pts, [0.18, 0.72], [0.16, 0.56], [0.0, 0.60], 26); // escote
    line(&mut pts, [0.0, 0.60], [0.0, 0.0], 24); // centro frente
    pts
}

impl ShapePipeline {
    /// Vértice de frontera más cercano a una fracción del perímetro
    /// (búsqueda binaria sobre las fracciones ordenadas, con wrap).
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

/// Empareja dos tramos de frontera para coserlos: `count` pares en
/// fracciones relativas iguales de cada tramo — con largos distintos, el
/// embebido (fruncido) emerge solo del desajuste. Para invertir la
/// dirección de un lado, pásalo con el rango al revés `(f1, f0)`.
/// Los índices de `b` llegan desplazados por `b_offset` (estado combinado).
pub fn pair_seam(
    a: &ShapePipeline,
    range_a: (f64, f64),
    b: &ShapePipeline,
    range_b: (f64, f64),
    b_offset: u32,
    count: usize,
) -> (Vec<u32>, Vec<u32>) {
    let mut va = Vec::with_capacity(count);
    let mut vb = Vec::with_capacity(count);
    for k in 0..count {
        let t = k as f64 / (count - 1) as f64;
        let fa = range_a.0 + (range_a.1 - range_a.0) * t;
        let fb = range_b.0 + (range_b.1 - range_b.0) * t;
        let (pa, pb) = (
            a.boundary_vertex_near(fa),
            b.boundary_vertex_near(fb) + b_offset,
        );
        // El "más cercano" puede repetirse en tramos densos: sin duplicados.
        if va.last() == Some(&pa) || vb.last() == Some(&pb) {
            continue;
        }
        va.push(pa);
        vb.push(pb);
    }
    (va, vb)
}
