use super::{COMPLIANCE, Scene, Seams, ShapePipeline, Sim, State, ball, meshes, sim, sim_of};
use crate::couture::{HANG_STEP, hung_at};

/// Where the cloth is put, in metres: clear of the ball and well above the
/// ring.
const START: f32 = 0.5;

/// The ring the run is hung from, in metres.
const RING: f32 = 0.10;

/// The ball, with one run of cloth named as hung from [`RING`].
fn holding(at: Vec<u32>) -> Scene {
    Scene {
        hung: vec![hung_at(at, RING)],
        ..ball(0.15)
    }
}

/// The mesh held clear of the ball and stepped one substep at a time, so the
/// only thing between the cloth and the ring is what one substep does.
///
/// Every particle starts at the same place, where the distance solve leaves
/// them all alone — an edge of no length asks for no correction — so the height
/// the anchor pass reads is a number this test can predict rather than observe.
fn clear_of_it(pipe: &ShapePipeline) -> Sim {
    let mut state = State::new(pipe.pos2d.len());
    state.py.fill(START);
    sim_of(state, pipe.constraints(COMPLIANCE), Seams::default(), 1)
}

/// A garment hung from nothing publishes nothing about hanging.
///
/// The reading the fitting room draws, and the whole of why it is an option: a
/// scene with no run leaves the anchor pass returning zero, and a zero
/// published as a reading would have every product anybody has drawn so far
/// reporting that it sits exactly at rings it never named.
#[test]
fn a_frame_of_a_product_hung_from_nothing_says_nothing_about_hanging() {
    let (plain, _) = meshes();
    let mut sim = sim(&plain);
    assert!(sim.scene.hung.is_empty(), "nothing is held up");
    assert_eq!(sim.publish().hanging, None, "before a single substep");
    sim.tick();
    assert_eq!(sim.publish().hanging, None, "and after a whole tick");
}

/// A hung run publishes how many runs the body holds, and how far the furthest
/// of their vertices was still from its ring when the pull let go of it.
///
/// The number is the pull's own, which is what makes it predictable: the cloth
/// stands [`START`] up, the ring is at [`RING`], the pull closes one
/// [`HANG_STEP`] of that, and the fall of one substep is the rest of the
/// difference.
#[test]
fn a_frame_of_a_hung_product_carries_what_the_pull_left() {
    let (plain, _) = meshes();
    let mut sim = clear_of_it(&plain);
    let every: Vec<u32> = (0..plain.pos2d.len() as u32).collect();
    assert_eq!(sim.apply_collider(1, holding(every)), Ok(()));
    assert_eq!(sim.publish().hanging, None, "not until a pass has run");

    sim.tick();
    let read = sim.publish().hanging.expect("the body holds a run");
    assert_eq!(read.runs, 1, "one run named, one run reported");
    assert_eq!(sim.substeps, 1, "one substep, one reading");
    let left = START - RING - HANG_STEP;
    assert!(
        (read.gap - left).abs() < 1.0e-3,
        "the pull left the cloth {} m from the ring, not {left} m",
        read.gap,
    );

    // A run of nothing is a run holding nothing, and it reads as the zero it
    // is: the count comes from the scene and the distance from the pass.
    assert_eq!(sim.apply_collider(2, holding(Vec::new())), Ok(()));
    sim.tick();
    let read = sim.publish().hanging.expect("the scene still names a run");
    assert_eq!((read.runs, read.gap), (1, 0.0));
}

/// A message that changes what is held drops the reading with it.
///
/// Every one of them can move the vertices or the ring, so a figure measured
/// against the scene before it is a figure about a garment nobody is looking at
/// any more — and published beside the new one it would be read as its own.
#[test]
fn a_reading_never_outlives_the_scene_it_was_taken_against() {
    let (plain, _) = meshes();
    let mut sim = clear_of_it(&plain);
    assert_eq!(sim.apply_collider(1, holding(vec![0, 1])), Ok(()));
    sim.tick();
    assert!(sim.publish().hanging.is_some(), "a reading was taken");

    let held = plain.constraints(COMPLIANCE);
    assert_eq!(
        sim.apply_rests(
            2,
            &held.rest,
            &held.compliance,
            (Vec::new(), 0),
            (Seams::default(), Vec::new())
        ),
        Ok(())
    );
    assert_eq!(sim.publish().hanging, None, "and the new scene holds none");
}
