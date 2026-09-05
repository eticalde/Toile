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
        SdfGrid::sphere(8, 0.25, [-1.0, -1.0, -1.0], [0.0, 0.0, 0.0], 0.15),
        pipe.tris.clone(),
        1.0 / 600.0,
        10,
    )
}

/// The rest lengths of one mesh must never be copied onto another. The
/// generation says which mesh a message was compiled against, and a
/// message from before the last swap is refused rather than taken.
#[test]
fn a_stale_generation_is_an_error_not_a_warm_start() {
    let (old, new) = meshes();
    let stale: Vec<f32> = old.constraints(COMPLIANCE).rest;
    let mut sim = sim(&old);
    assert_eq!(sim.apply_rests(1, &stale), Ok(()));

    let swap = Box::new(MeshSwap::new(&old.pos2d, &old.tris, &new, COMPLIANCE));
    let edges = swap.cons.rest.len();
    assert_eq!(sim.apply_swap(2, swap), Ok(()));
    assert_eq!(sim.state.len(), new.pos2d.len());
    assert_eq!(sim.cons.rest.len(), edges);

    // The rest update that was in flight when the swap landed.
    assert_eq!(
        sim.apply_rests(1, &stale),
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
    let refused = sim.apply_rests(1, &old.constraints(COMPLIANCE).rest);
    assert_eq!(
        refused,
        Err(StaleMessage::RestCount {
            expected,
            got: old.edges.len()
        })
    );
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
