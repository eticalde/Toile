use toile_sim::xpbd::Hung;

/// Most one substep may move a hung vertex toward its ring, in metres.
///
/// A dose and not a stiffness: the anchor is hard, so this cap is the whole of
/// what governs how fast it pulls, and the whole of the choice is here.
///
/// A ceiling and a floor, and neither is round. The ceiling is `bake::BAND`,
/// for that band's own reason: a constraint that moves a particle further than
/// the band the field was baked to can put it out past the band inside one
/// substep, where the field is saturated flat and no contact solve has a normal
/// to carry it back along. That is the fault the seams' own cap answers for,
/// and the two scenes in `tests/seeding/hang.rs` are what say this cap does not
/// reopen it: both insist on nothing buried, through the drape and at rest.
///
/// The floor is that the anchor has to arrive. Measured on the reference skirt,
/// two millimetres leaves the waistline five centimetres below the ring it is
/// held to — the cloth drags it down faster than the cap lifts it — and five
/// brings it to the ring.
pub const HANG_STEP: f32 = 0.005;

/// One run of cloth hung from a ring `height` metres up, as the solver holds
/// it.
///
/// The one place the cap is written onto a run, so a run built when the
/// product is let go and a run built after a rebuild cannot come to disagree
/// about how hard the body holds the garment.
pub fn hung_at(at: Vec<u32>, height: f32) -> Hung {
    Hung {
        at,
        height,
        max_step: HANG_STEP,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, reason = "a cap is carried over unchanged")]

    use super::*;
    use crate::body::bake;

    /// The cap the burial fault turns on: under the band, with room to spare,
    /// and a cap of nothing would hold nothing.
    #[test]
    fn the_step_stands_under_the_band_the_field_was_baked_to() {
        const { assert!(HANG_STEP > 0.0) };
        assert!(f64::from(HANG_STEP) < bake::BAND, "{HANG_STEP}");
    }

    #[test]
    fn a_run_carries_the_one_cap_and_the_height_it_was_given() {
        let run = hung_at(vec![4, 5, 6], 0.4064);
        assert_eq!(run.at, [4, 5, 6]);
        assert_eq!(run.height, 0.4064);
        assert_eq!(run.max_step, HANG_STEP);
    }
}
