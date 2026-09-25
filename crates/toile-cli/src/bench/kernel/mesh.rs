use std::time::Instant;

use toile_mesh::cdt;

use crate::bench::scene::same_bits;

/// Largest triangle the refinement may leave, in square metres.
const MAX_AREA: f64 = 2.0e-5;

/// spade's CDT and refinement over a concave piece.
pub(super) fn run() {
    let contour = toile_engine::demo::bodice_contour();

    let t = Instant::now();
    let mesh = cdt::triangulate(&contour, MAX_AREA).expect("the demo contour is finite");
    let ms1 = t.elapsed().as_secs_f64() * 1000.0;

    let t = Instant::now();
    let mesh2 = cdt::triangulate(&contour, MAX_AREA).expect("the demo contour is finite");
    let ms2 = t.elapsed().as_secs_f64() * 1000.0;

    let (h1, h2) = (cdt::mesh_hash(&mesh), cdt::mesh_hash(&mesh2));
    println!(
        "\n── spade CDT+refinement · contorno {} pts · pieza cóncava ──",
        contour.len()
    );
    println!(
        "malla            {} vértices · {} triángulos",
        mesh.vertices.len(),
        mesh.triangles.len() / 3
    );
    println!("triangulación    {ms1:7.3} ms (2ª corrida: {ms2:.3} ms)");
    println!(
        "hash             {h1:#018x}  reproducibilidad: {}",
        same_bits(h1, h2)
    );
}
