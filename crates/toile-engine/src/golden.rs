//! Escenario dorado de determinismo (ADR §2.4, Spike 2 — issue #34).
//!
//! Drapear → editar (vía A) → re-drapear, todo por conteo de substeps con
//! el solver escalar de referencia — sin reloj de pared, sin hilos. El
//! hash resultante debe ser idéntico en toda corrida; el test en CI lo
//! compara contra la constante commiteada en macOS ARM y Linux x86 a la
//! vez, así que también responde §3.3 (bit-exactitud cross-arquitectura).

use crate::couture::{ShapePipeline, demo_bodice_contour};
use toile_sim::xpbd::{self, SdfGrid, State};

/// Drapea el corpiño demo 600 substeps, mueve el vértice hombro-sisa
/// +2 cm (recompilación vía A con warm start) y drapea 600 más.
pub fn drape_bodice_hash() -> u64 {
    const DT: f32 = 1.0 / 600.0;
    let mut contour = demo_bodice_contour();
    let mut pipe = ShapePipeline::build(&contour, 256, 2.0e-5);

    let n = pipe.pos2d.len();
    let (mut cx, mut cy) = (0.0, 0.0);
    for p in &pipe.pos2d {
        cx += p[0];
        cy += p[1];
    }
    cx /= n as f64;
    cy /= n as f64;
    let mut state = State::new(n);
    for i in 0..n {
        state.px[i] = (pipe.pos2d[i][0] - cx) as f32;
        state.py[i] = 0.35;
        state.pz[i] = (pipe.pos2d[i][1] - cy) as f32;
    }
    let mut cons = pipe.constraints(1.0e-8);
    let sdf = SdfGrid::sphere(256, 1.4 / 255.0, [-0.7, -0.7, -0.7], [0.0, 0.0, 0.0], 0.15);

    for _ in 0..600 {
        xpbd::substep(&mut state, &cons, &sdf, DT);
    }
    contour[68][0] += 0.02;
    cons.rest.copy_from_slice(pipe.derive(&contour));
    for _ in 0..600 {
        xpbd::substep(&mut state, &cons, &sdf, DT);
    }
    xpbd::position_hash(&state)
}
