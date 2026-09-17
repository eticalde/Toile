use std::sync::Arc;

use super::*;
use crate::couture::{COMPLIANCE, ShapePipeline};

/// A rectangle and the same rectangle with one node more.
fn meshes() -> (ShapePipeline, ShapePipeline) {
    let plain = [[0.0, 0.0], [0.30, 0.0], [0.30, 0.20], [0.0, 0.20]];
    let mut cut = plain.to_vec();
    cut.insert(1, [0.15, 0.0]);
    (
        ShapePipeline::build(&plain, 16, 0.01).expect("the rectangle is finite"),
        ShapePipeline::build(&cut, 17, 0.01).expect("the rectangle is finite"),
    )
}

fn sim(pipe: &ShapePipeline) -> Sim {
    Sim::new(
        State::new(pipe.pos2d.len()),
        pipe.constraints(COMPLIANCE),
        Seams::default(),
        ball(0.15),
        pipe.tris.clone(),
        1.0 / 600.0,
        10,
    )
}

/// A sphere at the origin, as the sim thread takes one.
fn ball(radius: f32) -> Arc<SdfGrid> {
    Arc::new(SdfGrid::sphere(
        8,
        0.25,
        [-1.0, -1.0, -1.0],
        [0.0, 0.0, 0.0],
        radius,
    ))
}

/// One pair sewn between the two vertices named.
fn sewn(a: u32, b: u32) -> Seams {
    Seams {
        a: vec![a],
        b: vec![b],
        compliance: 1.0e-5,
        max_step: 0.002,
        iterations: 4,
    }
}

/// The rest lengths of one mesh must never be copied onto another. The
/// generation says which mesh a message was compiled against, and a
/// message from before the last swap is refused rather than taken.
#[test]
fn a_stale_generation_is_an_error_not_a_warm_start() {
    let (old, new) = meshes();
    let stale: Vec<f32> = old.constraints(COMPLIANCE).rest;
    let mut sim = sim(&old);
    assert_eq!(sim.apply_rests(1, &stale, Seams::default()), Ok(()));

    let swap = Box::new(MeshSwap::new(&old.pos2d, &old.tris, &new, COMPLIANCE));
    let edges = swap.cons.rest.len();
    assert_eq!(sim.apply_swap(2, swap), Ok(()));
    assert_eq!(sim.state.len(), new.pos2d.len());
    assert_eq!(sim.cons.rest.len(), edges);

    // The rest update that was in flight when the swap landed.
    assert_eq!(
        sim.apply_rests(1, &stale, Seams::default()),
        Err(StaleMessage::Generation { applied: 2, got: 1 })
    );
    assert_eq!(sim.cons.rest.len(), edges, "the new mesh kept its rests");
}

/// The count check is the second gate: a fresh generation carrying the
/// wrong number of rest lengths would have overrun the buffer.
#[test]
fn rest_lengths_for_another_mesh_are_refused_by_count() {
    let (old, new) = meshes();
    let mut sim = sim(&new);
    let expected = sim.cons.rest.len();
    let stale = old.constraints(COMPLIANCE).rest;
    let refused = sim.apply_rests(1, &stale, Seams::default());
    assert_eq!(
        refused,
        Err(StaleMessage::RestCount {
            expected,
            got: old.edges.len()
        })
    );
}

/// A swap naming a run the solver does not hold is refused rather than
/// taken. Slicing it would panic on the sim thread, which stops the drape
/// with nothing said anywhere a person could read it.
#[test]
fn a_swap_past_the_end_of_the_state_is_refused() {
    let (old, new) = meshes();
    let mut sim = sim(&old);
    let held = old.pos2d.len();
    let swap = Box::new(MeshSwap {
        at: 1,
        replacing: held as u32,
        ..MeshSwap::new(&old.pos2d, &old.tris, &new, COMPLIANCE)
    });
    assert_eq!(
        sim.apply_swap(1, swap),
        Err(StaleMessage::SwapRange {
            at: 1,
            replacing: held as u32,
            len: held,
        })
    );
    assert_eq!(
        sim.state.len(),
        held,
        "the mesh it had is still on the stand"
    );
}

/// A seam is a pair of raw indices into the state, so one past the end is a
/// panic on the sim thread rather than a wrong drape. It is refused on the
/// way in, and the cloth goes on unsewn rather than stopping.
#[test]
fn a_seam_past_the_end_of_the_state_is_refused() {
    let (old, _) = meshes();
    let mut sim = sim(&old);
    let held = old.pos2d.len();
    let rests = old.constraints(COMPLIANCE).rest;
    assert_eq!(
        sim.apply_rests(1, &rests, sewn(0, held as u32)),
        Err(StaleMessage::SeamRange {
            vertex: held as u32,
            len: held,
        })
    );
    assert!(sim.seams.is_empty(), "nothing was sewn");
}

/// A swap is judged against the state it leaves behind, not the one it
/// replaces: its seams are written in the new numbering, and a piece that
/// gained vertices makes indices legal that would have been past the end.
#[test]
fn a_swap_judges_its_seams_against_the_state_it_leaves() {
    let (old, new) = meshes();
    let mut sim = sim(&old);
    let grew = new.pos2d.len() - old.pos2d.len();
    assert!(grew > 0, "the rebuilt mesh really is larger");
    let past = (new.pos2d.len() - 1) as u32;
    let swap = Box::new(MeshSwap {
        seams: sewn(0, past),
        ..MeshSwap::new(&old.pos2d, &old.tris, &new, COMPLIANCE)
    });
    assert_eq!(sim.apply_swap(1, swap), Ok(()));
    assert_eq!(sim.seams.b, [past], "and the seam came with it");
}

/// A swap only ever lands between substeps, and the drape it carries is
/// the one the solver had: same height, same velocities.
#[test]
fn a_swap_between_ticks_keeps_the_cloth_where_it_was() {
    let (old, new) = meshes();
    let mut sim = sim(&old);
    for i in 0..sim.state.len() {
        sim.state.py[i] = 0.30;
        sim.state.vy[i] = -0.5;
    }
    sim.tick();
    let before = mean_height(&sim.state);
    let swap = Box::new(MeshSwap::new(&old.pos2d, &old.tris, &new, COMPLIANCE));
    assert_eq!(sim.apply_swap(1, swap), Ok(()));
    assert!((mean_height(&sim.state) - before).abs() < 1.0e-3);
    assert!(!sim.converged(), "a swap wakes the cloth");
}

fn mean_height(state: &State) -> f32 {
    state.py.iter().sum::<f32>() / state.len() as f32
}
