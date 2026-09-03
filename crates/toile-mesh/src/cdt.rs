use spade::{ConstrainedDelaunayTriangulation, Point2, RefinementParameters, Triangulation};

/// A piece meshed in 2D: vertices in metres, CCW triangles.
#[allow(
    missing_docs,
    reason = "SoA buffers are named by their axis; a doc
    per field would only restate the name"
)]
#[derive(Debug, Clone, Default)]
pub struct PieceMesh {
    pub vertices: Vec<[f64; 2]>,
    pub triangles: Vec<u32>,
}

/// Triangulates the interior of a closed CCW contour, refining until every
/// triangle is under `max_area`.
///
/// The contour must not repeat its first point. Vertices are inserted in
/// contour order: that order is part of the determinism contract, since spade
/// resolves ties by insertion sequence.
///
/// A contour of fewer than three points has no interior and yields an empty
/// mesh.
///
/// # Panics
/// If a contour point is not representable as a spade coordinate — that is, if
/// it is infinite or NaN.
pub fn triangulate(contour: &[[f64; 2]], max_area: f64) -> PieceMesh {
    if contour.len() < 3 {
        return PieceMesh::default();
    }
    let mut cdt = ConstrainedDelaunayTriangulation::<Point2<f64>>::new();
    let handles: Vec<_> = contour
        .iter()
        .map(|p| {
            cdt.insert(Point2::new(p[0], p[1]))
                .expect("contour points must be finite")
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
    // spade meshes the whole convex hull, so the faces filling a concavity
    // (an armhole, a neckline) have to be dropped. Centroid even-odd rather
    // than spade's own outer-face marking: it is exact on f64 and stable.
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

/// FNV-1a over coordinate bits and indices — the reproducibility golden.
pub fn mesh_hash(mesh: &PieceMesh) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |x: u64| h = (h ^ x).wrapping_mul(0x0100_0000_01b3);
    for v in &mesh.vertices {
        eat(v[0].to_bits());
        eat(v[1].to_bits());
    }
    for &t in &mesh.triangles {
        eat(u64::from(t));
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(step: f64) -> Vec<[f64; 2]> {
        let n = (1.0 / step) as usize;
        let mut pts = Vec::new();
        for (dx, dy, ox, oy) in [
            (1.0, 0.0, 0.0, 0.0),
            (0.0, 1.0, 1.0, 0.0),
            (-1.0, 0.0, 1.0, 1.0),
            (0.0, -1.0, 0.0, 1.0),
        ] {
            for i in 0..n {
                let t = i as f64 / n as f64;
                pts.push([ox + dx * t, oy + dy * t]);
            }
        }
        pts
    }

    #[test]
    fn a_square_meshes_to_a_covered_interior() {
        let m = triangulate(&square(0.1), 0.02);
        assert!(!m.triangles.is_empty());
        assert!(m.triangles.len().is_multiple_of(3));
        let area: f64 = m
            .triangles
            .chunks(3)
            .map(|t| {
                let (a, b, c) = (
                    m.vertices[t[0] as usize],
                    m.vertices[t[1] as usize],
                    m.vertices[t[2] as usize],
                );
                ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])).abs() / 2.0
            })
            .sum();
        assert!((area - 1.0).abs() < 1.0e-9, "covered area was {area}");
    }

    #[test]
    fn refinement_respects_max_area() {
        let m = triangulate(&square(0.25), 0.01);
        for t in m.triangles.chunks(3) {
            let (a, b, c) = (
                m.vertices[t[0] as usize],
                m.vertices[t[1] as usize],
                m.vertices[t[2] as usize],
            );
            let area = ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])).abs() / 2.0;
            assert!(area <= 0.01 + 1.0e-12, "triangle of area {area}");
        }
    }

    #[test]
    fn the_same_contour_meshes_to_the_same_bits() {
        let c = square(0.1);
        assert_eq!(
            mesh_hash(&triangulate(&c, 0.02)),
            mesh_hash(&triangulate(&c, 0.02))
        );
    }

    #[test]
    fn a_contour_without_an_interior_yields_an_empty_mesh() {
        assert!(triangulate(&[], 0.1).triangles.is_empty());
        assert!(
            triangulate(&[[0.0, 0.0], [1.0, 0.0]], 0.1)
                .triangles
                .is_empty()
        );
    }
}
