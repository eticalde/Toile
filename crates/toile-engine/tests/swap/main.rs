#![allow(missing_docs, reason = "a test crate publishes no API surface")]
#![allow(clippy::float_cmp, reason = "a round centimetre is landed on exactly")]

/// The other half of the swap: the session goes on drafting while the worker
/// rebuilds the mesh behind it.
mod remesh;

use std::time::Duration;

use toile_engine::couture::{self, COMPLIANCE, MeshSwap, ShapePipeline};
use toile_sim::xpbd::{self, DistanceConstraints, SdfGrid, Seams, Stage, State};

/// How long a test waits on another thread before calling it stuck.
pub const PATIENCE: Duration = Duration::from_secs(20);

/// Simulated seconds per substep, as the engine runs it.
const DT: f32 = 1.0 / 600.0;

/// Substeps of drape before the topology changes: long enough for the panel
/// to reach the sphere and still be moving when the mesh is pulled from under
/// it, which is the moment the swap has to survive.
const DRAPE: usize = 240;

/// Substeps of settling allowed after the swap, a tenth of a second.
const SETTLE: usize = 60;

/// How much motion one swap may add, as a multiple of the motion the drape
/// already had. The topology benchmark measures 2.6 on the demo bodice; this
/// is the ceiling that separates a re-projection from a detonation.
const SPIKE: f32 = 4.0;

/// A panel, and the same panel after a node was put into its hem and pulled
/// a centimetre out: the topology edit the phase is named for, meshed at the
/// density the engine itself chooses.
fn panels() -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
    let plain = vec![[0.0, 0.0], [0.20, 0.0], [0.20, 0.30], [0.0, 0.30]];
    let mut cut = plain.clone();
    cut.insert(1, [0.10, -0.01]);
    (plain, cut)
}

/// Meshes a contour the way the engine does, density and all.
fn mesh(contour: &[[f64; 2]]) -> ShapePipeline {
    let (samples, max_area) = couture::for_contour(contour);
    ShapePipeline::build(contour, samples, max_area).expect("the panel is finite")
}

/// The avatar, small enough that a test does not bake a 67 MB grid.
fn sphere() -> SdfGrid {
    SdfGrid::sphere(48, 0.5 / 47.0, [-0.25, -0.25, -0.25], [0.0, 0.0, 0.0], 0.08)
}

/// `n` substeps of the reference scalar path: no ground, nothing sewn.
fn drape(state: &mut State, cons: &DistanceConstraints, sdf: &SdfGrid, n: usize) {
    let seams = Seams::default();
    for _ in 0..n {
        xpbd::substep(state, cons, &seams, &Stage::around(sdf), None, DT);
    }
}

/// Mean kinetic energy per vertex: the drape's motion, in one number.
fn energy(state: &State) -> f32 {
    xpbd::kinetic_energy(state) / state.len() as f32
}

/// Mean height, in metres: where the cloth is, in one number.
fn height(state: &State) -> f32 {
    state.py.iter().sum::<f32>() / state.len() as f32
}

/// Lowest and highest vertex, in metres.
fn span(state: &State) -> (f32, f32) {
    state
        .py
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), &y| (lo.min(y), hi.max(y)))
}

/// A mid-air drape carried onto a re-meshed panel keeps its position and its
/// motion: the transfer moves cloth, it does not stir it.
#[test]
fn the_drape_survives_a_mesh_swap() {
    let (plain, cut) = panels();
    let old = mesh(&plain);
    let new = mesh(&cut);
    assert_ne!(old.pos2d.len(), new.pos2d.len(), "the mesh really changed");
    let cons = old.constraints(COMPLIANCE);
    let sdf = sphere();

    let mut state = couture::drop_state(&old, couture::DROP_HEIGHT);
    drape(&mut state, &cons, &sdf, DRAPE);
    let (was_moving, was_at) = (energy(&state), height(&state));
    let (was_lowest, was_highest) = span(&state);
    assert!(
        was_moving > 1.0e-4,
        "the panel is still draping: {was_moving}"
    );

    let swap = MeshSwap::new(&old.pos2d, &old.tris, &new, COMPLIANCE);
    let mut carried = couture::onto(&swap, &state);
    assert_eq!(carried.len(), new.pos2d.len(), "the new mesh is in place");
    // The new mesh samples the old cloth at its own vertices, so the mean
    // moves a little; what it must not do is invent a fold. Every carried
    // position is a convex mixture of three old ones, and stays inside them.
    let (lowest, highest) = span(&carried);
    assert!(
        lowest >= was_lowest - 1.0e-6 && highest <= was_highest + 1.0e-6,
        "the swap invented no new extreme: {lowest}..{highest} against {was_lowest}..{was_highest}"
    );
    assert!(
        (height(&carried) - was_at).abs() < 5.0e-3,
        "the cloth stayed where it was: {} against {was_at}",
        height(&carried)
    );
    assert!(
        energy(&carried) < was_moving * 1.5,
        "the swap added no motion: {} against {was_moving}",
        energy(&carried)
    );

    // The first substep on the new mesh pays for the interpolation: the
    // carried surface is the old one's chords, so the stretch constraints
    // pull it back onto its rest lengths. It is a step, not a bang, and it is
    // paid once — a tenth of a second later the panel is quieter than it was
    // before the swap, which is the drape carrying on rather than restarting.
    drape(&mut carried, &swap.cons, &sdf, 1);
    let spike = energy(&carried);
    assert!(
        spike < was_moving * SPIKE,
        "the swap cost one bounded step: {spike} against {was_moving}"
    );
    drape(&mut carried, &swap.cons, &sdf, SETTLE);
    assert!(
        energy(&carried) < was_moving,
        "and the panel went on settling: {} against {was_moving}",
        energy(&carried)
    );
}
