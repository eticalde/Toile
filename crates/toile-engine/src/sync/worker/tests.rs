#![allow(
    clippy::float_cmp,
    reason = "a plane handed to the sim comes back at the very bits it was given"
)]

use std::sync::Arc;

use toile_sim::xpbd::{Floor, SdfGrid};

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

/// A sphere at the origin standing on nothing, as the sim thread takes one.
///
/// No floor, because every test here is about what the mailbox does with a
/// message: a ground would add a pass none of them measures.
fn ball(radius: f32) -> Scene {
    Scene {
        sdf: Arc::new(SdfGrid::sphere(
            8,
            0.25,
            [-1.0, -1.0, -1.0],
            [0.0, 0.0, 0.0],
            radius,
        )),
        floor: Floor::none(),
    }
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
    let held = old.constraints(COMPLIANCE);
    let (stale, firm) = (held.rest, held.compliance);
    let mut sim = sim(&old);
    assert_eq!(sim.apply_rests(1, &stale, &firm, Seams::default()), Ok(()));

    let swap = Box::new(MeshSwap::new(&old.pos2d, &old.tris, &new, COMPLIANCE));
    let edges = swap.cons.rest.len();
    assert_eq!(sim.apply_swap(2, swap), Ok(()));
    assert_eq!(sim.state.len(), new.pos2d.len());
    assert_eq!(sim.cons.rest.len(), edges);

    // The rest update that was in flight when the swap landed.
    assert_eq!(
        sim.apply_rests(1, &stale, &firm, Seams::default()),
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
    let held = old.constraints(COMPLIANCE);
    let refused = sim.apply_rests(1, &held.rest, &held.compliance, Seams::default());
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
    let firm = old.constraints(COMPLIANCE);
    assert_eq!(
        sim.apply_rests(1, &firm.rest, &firm.compliance, sewn(0, held as u32)),
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

/// The plane a body of this height would stand on.
const PLANE: f32 = -0.837;

/// A body arriving brings its own ground with it.
///
/// The field and the plane are one message because they are one body. Taking
/// the field and keeping the ground the body before it stood on would leave
/// the drape resting on a plane that belongs to nothing on the stand — which
/// over a shorter body is a garment lying in the air below its feet.
#[test]
fn a_body_arriving_brings_its_own_ground() {
    let (old, _) = meshes();
    let mut sim = sim(&old);
    assert_eq!(
        sim.scene.floor.level(),
        None,
        "the fixture stands on nothing"
    );
    let standing = Scene {
        sdf: ball(0.10).sdf,
        floor: Floor::at(PLANE),
    };
    assert_eq!(sim.apply_collider(1, standing), Ok(()));
    assert_eq!(sim.scene.floor.level(), Some(PLANE));
    assert!(!sim.converged(), "a body wakes the cloth");
}

fn mean_height(state: &State) -> f32 {
    state.py.iter().sum::<f32>() / state.len() as f32
}

/// Sleep is a pause and never an ending: the message that arrives next finds
/// the drape awake before another tick is run, whatever it slept on.
#[test]
fn a_sleeping_drape_is_awake_the_moment_a_message_lands() {
    let (old, _) = meshes();
    let mut sim = sim(&old);
    // Held where it is and clear of the ball, so nothing moves and the dwell
    // is all there is to wait for.
    sim.state.inv_mass.fill(0.0);
    sim.state.py.fill(0.5);
    for _ in 0..1000 {
        if sim.converged() {
            break;
        }
        sim.tick();
    }
    assert!(sim.converged(), "cloth that cannot move goes to sleep");
    assert!(
        sim.publish().converged,
        "and the frame it publishes says so"
    );

    assert_eq!(sim.apply_collider(1, ball(0.10)), Ok(()));
    assert!(!sim.converged(), "a body wakes it");
    sim.tick();
    assert!(!sim.converged(), "and one quiet tick does not put it back");
}
