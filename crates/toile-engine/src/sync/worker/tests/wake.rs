use super::{ball, meshes, sim};

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
