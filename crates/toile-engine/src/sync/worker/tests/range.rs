use toile_sim::xpbd::Seams;

use super::{StaleMessage, meshes, sewn, sim};
use crate::couture::{COMPLIANCE, hung_at};

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
        sim.apply_rests(
            1,
            &firm.rest,
            &firm.compliance,
            (Vec::new(), 0),
            (sewn(0, held as u32), Vec::new())
        ),
        Err(StaleMessage::SeamRange {
            vertex: held as u32,
            len: held,
        })
    );
    assert!(sim.seams.is_empty(), "nothing was sewn");
}

/// And a hang is the same kind of index, refused the same way: the anchor pass
/// reads the state directly, and the drape goes on holding nothing rather than
/// stopping on the sim thread.
#[test]
fn a_hang_past_the_end_of_the_state_is_refused() {
    let (old, _) = meshes();
    let mut sim = sim(&old);
    let held = old.pos2d.len();
    let firm = old.constraints(COMPLIANCE);
    let past = hung_at(vec![held as u32], 0.4);
    assert_eq!(
        sim.apply_rests(
            1,
            &firm.rest,
            &firm.compliance,
            (Vec::new(), 0),
            (Seams::default(), vec![past])
        ),
        Err(StaleMessage::HungRange {
            vertex: held as u32,
            len: held,
        })
    );
    assert!(sim.scene.hung.is_empty(), "nothing is held up");
}
