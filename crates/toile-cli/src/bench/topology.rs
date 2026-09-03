use std::time::Instant;

use toile_engine::couture::{ShapePipeline, transfer_state};
use toile_engine::demo;
use toile_mesh::transfer;
use toile_sim::xpbd::{self, Seams};

use super::scene::{DT, same_bits, seconds, settle};

/// Substeps of drape before the topology change.
const DRAPE_SUBSTEPS: usize = 600;

struct Swap {
    rebuild_ms: f64,
    energy_before: f64,
    energy_after: f64,
    reconverge_s: f64,
    hash: u64,
}

/// Adds a contour point and moves the shoulder, then carries the live drape
/// onto the new mesh.
///
/// In the engine this rebuild runs in the shadow of the old mesh while the
/// solver keeps integrating; here it is measured on its own.
fn swap() -> Swap {
    let no_seams = Seams::default();
    let contour_a = demo::bodice_contour();
    let pipe_a = demo::pipeline(&contour_a);
    let mut state = demo::drop_state(&pipe_a);
    let cons_a = pipe_a.constraints(1.0e-8);
    let sdf = demo::avatar_sdf();
    for _ in 0..DRAPE_SUBSTEPS {
        xpbd::substep(&mut state, &cons_a, &no_seams, &sdf, DT);
    }
    let n = state.len();
    let energy_before = f64::from(xpbd::kinetic_energy(&state) / n as f32);

    let mut contour_b = contour_a.clone();
    let mid = [
        f64::midpoint(contour_b[29][0], contour_b[30][0]) + 0.02,
        f64::midpoint(contour_b[29][1], contour_b[30][1]),
    ];
    contour_b.insert(30, mid);
    // The insert pushed every later index along by one.
    contour_b[demo::SHOULDER_POINT + 1][0] += 0.03;

    let t0 = Instant::now();
    let pipe_b = demo::pipeline(&contour_b);
    let cons_b = pipe_b.constraints(1.0e-8);
    let mut state_b = transfer_state(&pipe_a, &state, &pipe_b);
    let rebuild_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let nb = state_b.len();

    xpbd::substep(&mut state_b, &cons_b, &no_seams, &sdf, DT);
    let energy_after = f64::from(xpbd::kinetic_energy(&state_b) / nb as f32);

    let steps = settle(&mut state_b, &cons_b, &no_seams, &sdf, 6000);
    Swap {
        rebuild_ms,
        energy_before,
        energy_after,
        reconverge_s: seconds(steps),
        hash: xpbd::position_hash(&state_b),
    }
}

/// How badly the interior interpolator folds under extreme edits.
fn foldovers() {
    let contour = demo::bodice_contour();
    let edits: [(&str, usize, [f64; 2]); 3] = [
        (
            "hombro +2 cm (suave)     ",
            demo::SHOULDER_POINT,
            [0.02, 0.0],
        ),
        (
            "hombro -10 cm (extrema)  ",
            demo::SHOULDER_POINT,
            [-0.10, 0.0],
        ),
        ("sisa +12 cm (a través)   ", 50, [0.12, 0.0]),
    ];
    println!("\n── foldovers del interpolador (MVC) en ediciones extremas ──");
    for (name, idx, d) in edits {
        let mut pipe = demo::pipeline(&contour);
        let reference = pipe.pos2d.clone();
        let tris = pipe.tris.clone();
        let mut edited = contour.clone();
        edited[idx][0] += d[0];
        edited[idx][1] += d[1];
        pipe.derive(&edited);
        let flips = transfer::count_flipped(&reference, &pipe.pos2d, &tris);
        println!(
            "{name}  {flips} triángulos invertidos de {}  {}",
            tris.len() / 3,
            if flips == 0 { "✅" } else { "⚠️" }
        );
    }
}

fn mesh_hash(p: &ShapePipeline) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |x: u64| h = (h ^ x).wrapping_mul(0x0100_0000_01b3);
    for v in &p.pos2d {
        eat(v[0].to_bits());
        eat(v[1].to_bits());
    }
    for &t in &p.tris {
        eat(u64::from(t));
    }
    h
}

pub fn run() {
    println!("\n── vía B: topología con transferencia baricéntrica ──");
    let a = swap();
    let b = swap();
    println!(
        "rebuild + transferencia {:7.1} ms  (presupuesto: <500 ms)",
        a.rebuild_ms
    );
    println!(
        "energía/vért            {:9.2e} antes del swap · {:9.2e} tras el primer substep",
        a.energy_before, a.energy_after
    );
    println!(
        "re-convergencia         {:7.2} s de sim tras el swap",
        a.reconverge_s
    );
    println!("determinismo            {}", same_bits(a.hash, b.hash));

    foldovers();

    // Undo rebuilds from the original contour: the sampling map lives in the
    // document revision, so the mesh has to come back bit-exact.
    let contour = demo::bodice_contour();
    let (m1, m2) = (
        mesh_hash(&demo::pipeline(&contour)),
        mesh_hash(&demo::pipeline(&contour)),
    );
    println!(
        "\nundo (rebuild del contorno original): malla {}",
        if m1 == m2 {
            "bit-idéntica ✅"
        } else {
            "DIFIERE ❌"
        }
    );
}
