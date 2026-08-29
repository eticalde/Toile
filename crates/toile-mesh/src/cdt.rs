//! CDT + Delaunay refinement por pieza — Spike 1 (issue #33).
//!
//! Inserción en orden canónico (el orden del contorno es parte del contrato
//! de determinismo: misma pieza → misma malla, siempre).

use spade::{ConstrainedDelaunayTriangulation, Point2, RefinementParameters, Triangulation};

/// Malla 2D de una pieza: vértices en metros y triángulos CCW.
pub struct PieceMesh {
    pub vertices: Vec<[f64; 2]>,
    pub triangles: Vec<u32>,
}

/// Triangula el interior de un contorno cerrado (polilínea CCW, sin el
/// punto de cierre duplicado) con refinement hasta `max_area` por triángulo.
pub fn triangulate(contour: &[[f64; 2]], max_area: f64) -> PieceMesh {
    let mut cdt = ConstrainedDelaunayTriangulation::<Point2<f64>>::new();
    let handles: Vec<_> = contour
        .iter()
        .map(|p| {
            cdt.insert(Point2::new(p[0], p[1]))
                .expect("vértice inválido")
        })
        .collect();
    for i in 0..handles.len() {
        cdt.add_constraint(handles[i], handles[(i + 1) % handles.len()]);
    }
    let params = RefinementParameters::<f64>::new()
        .exclude_outer_faces(true)
        .with_max_allowed_area(max_area)
        .with_max_additional_vertices(50_000);
    cdt.refine(params);

    let mut vertices = Vec::with_capacity(cdt.num_vertices());
    for v in cdt.vertices() {
        let p = v.position();
        vertices.push([p.x, p.y]);
    }
    // spade triangula todo el hull convexo; las caras del hull que quedan
    // fuera del contorno (la concavidad de una sisa) se filtran por
    // centroide con even-odd — determinista.
    let mut triangles = Vec::new();
    for f in cdt.inner_faces() {
        let [a, b, c] = f.vertices().map(|v| v.index() as u32);
        let centroid = [
            (vertices[a as usize][0] + vertices[b as usize][0] + vertices[c as usize][0]) / 3.0,
            (vertices[a as usize][1] + vertices[b as usize][1] + vertices[c as usize][1]) / 3.0,
        ];
        if point_in_polygon(centroid, contour) {
            triangles.push(a);
            triangles.push(b);
            triangles.push(c);
        }
    }
    PieceMesh {
        vertices,
        triangles,
    }
}

/// Even-odd ray casting, f64 determinista.
fn point_in_polygon(p: [f64; 2], poly: &[[f64; 2]]) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Hash FNV-1a de la malla completa (bits de coordenadas + índices):
/// el golden de reproducibilidad de spade.
pub fn mesh_hash(mesh: &PieceMesh) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    let mut eat = |x: u64| h = (h ^ x).wrapping_mul(0x100000001b3);
    for v in &mesh.vertices {
        eat(v[0].to_bits());
        eat(v[1].to_bits());
    }
    for &t in &mesh.triangles {
        eat(u64::from(t));
    }
    h
}
