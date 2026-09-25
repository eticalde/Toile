#![allow(
    clippy::float_cmp,
    reason = "a particle nothing moved is at the very bits it was handed in at"
)]

use super::*;

/// The saturated magnitude a bake writes outside its narrow band.
const BAND: f32 = 0.025;

/// Half the spacing between samples: the step the gradient is read over.
const EPS: f32 = 0.0025;

/// A field with a saturated interior, as a baked body has: a block of samples
/// all at `-BAND`, everything around it at `+BAND`, and no ramp between them.
fn saturated() -> SdfGrid {
    let dims = [8, 8, 8];
    let mut data = vec![BAND; dims[0] * dims[1] * dims[2]];
    for k in 2..6 {
        for j in 2..6 {
            for i in 2..6 {
                data[(k * dims[1] + j) * dims[0] + i] = -BAND;
            }
        }
    }
    SdfGrid {
        dims,
        cell: 2.0 * EPS,
        origin: [0.0; 3],
        data,
    }
}

/// A particle whose whole step lay deep in the saturated interior is left
/// exactly where it is — with the motion it arrived carrying.
///
/// There is no normal anywhere on such a step. The push along a zero gradient
/// moves nothing, so the friction and the damping that follow would be spent
/// against a normal that does not exist: they would take away every bit of
/// tangential motion and stop the particle dead. The stretch constraints are
/// what draw it out, through the neighbours that do have a gradient, and they
/// cannot do it if the contact solve has already frozen it.
#[test]
fn a_particle_in_the_saturated_interior_is_left_to_the_constraints() {
    let sdf = saturated();
    let p = [0.015, 0.015, 0.015];
    assert_eq!(sdf.sample(p[0], p[1], p[2]), -BAND, "it is under the skin");
    let q = [0.010, 0.015, 0.015];
    assert_eq!(
        sdf.sample(q[0], q[1], q[2]),
        -BAND,
        "and so is where it began"
    );
    let (moved, before) = resolve(&sdf, EPS, Grip::slipping(), p, q);
    assert_eq!(moved, p, "nothing pushed it, so nothing may move it");
    assert_eq!(before, q, "and the motion it had is still there to be read");
}

/// A particle carried past the band inside one substep is brought back along
/// the step it took, and ends outside the body.
///
/// The one above is what this must not be confused with. Both end where the
/// field is saturated flat, but this one crossed the skin on the way in, and
/// the segment it travelled is a reading the contact solve has and the bare
/// position does not. Without it the particle stays: every later substep finds
/// it in the same flat nothing, so the drape settles with cloth inside a
/// person.
///
/// Begun on the ramp, because this field saturates outside its band as well as
/// inside: a step whose own start reads no gradient either says nothing about
/// where the skin was crossed, and is the case above by another route.
#[test]
fn a_particle_a_step_carried_past_the_band_is_brought_back_along_that_step() {
    let sdf = saturated();
    let p = [0.015, 0.015, 0.015];
    let q = [0.007, 0.015, 0.015];
    assert_eq!(
        sdf.sample(p[0], p[1], p[2]),
        -BAND,
        "it ended under the skin"
    );
    assert!(
        sdf.sample(q[0], q[1], q[2]) > 0.0,
        "and began clear of it, where the field can still be read"
    );
    let (moved, _) = resolve(&sdf, EPS, Grip::slipping(), p, q);
    assert!(
        sdf.sample(moved[0], moved[1], moved[2]) >= 0.0,
        "it is out of the body: {moved:?}"
    );
}

/// A particle the band does reach is still carried out of the body, which is
/// the case neither of the two above may have broken.
#[test]
fn a_particle_the_band_reaches_is_still_pushed_out() {
    let sdf = saturated();
    let p = [0.025, 0.015, 0.015];
    assert!(sdf.sample(p[0], p[1], p[2]) < 0.0, "it is under the skin");
    let (moved, _) = resolve(&sdf, EPS, Grip::slipping(), p, p);
    assert!(moved[0] > p[0], "pushed along the gradient: {moved:?}");
}

/// The plane a scene has no floor at.
const GROUND: f32 = -0.837;

/// Cloth above the ground is cloth in free flight, and the floor pass hands
/// it back at the very bits it was given.
///
/// This is what a garment still falling has to read as, all the way down to
/// the moment it touches: a floor that rounded a particle it never reached
/// would move a drape that never landed.
#[test]
fn a_particle_above_the_plane_is_handed_straight_back() {
    let p = [0.1, GROUND + 0.001, -0.2];
    let q = [0.3, GROUND + 0.004, -0.1];
    let (moved, before) = rest_on(GROUND, Grip::slipping(), p, q);
    assert_eq!(moved, p, "nothing under it, so nothing moved it");
    assert_eq!(before, q, "and the motion it had is still there to be read");
}

/// A particle that has gone under is set exactly on the plane, its sideways
/// motion cut, and `q` drawn down after it.
///
/// All three together are what makes a garment land rather than skate: the
/// plane stops the fall, the friction stops the slide, and moving `q` is how
/// `derive_velocities` is told the particle is no longer travelling.
#[test]
fn a_particle_under_the_plane_is_set_on_it_and_slowed() {
    let q = [0.0, GROUND + 0.010, 0.0];
    let p = [0.020, GROUND - 0.010, 0.0];
    let (moved, before) = rest_on(GROUND, Grip::slipping(), p, q);
    assert_eq!(moved[1], GROUND, "it is on the floor, not through it");
    assert!(
        moved[0] > q[0] && moved[0] < p[0],
        "the slide was cut, not reversed and not ignored: {moved:?}"
    );
    // PBD reads velocity as (p − q)/dt, so the way a landing loses its speed
    // is `q` being drawn toward `p` — down to meet the plane, never up.
    let fell = q[1] - moved[1];
    let left = before[1] - moved[1];
    assert!(
        left > 0.0 && left < fell,
        "q came down after it rather than away from it: {before:?}"
    );
    assert!(
        (left - fell * 0.5).abs() < 1.0e-6,
        "and came half the way, which is the contact damping: {left} of {fell}"
    );
}

/// A scene with no floor says so, and one with a floor carries the very
/// number it was given.
#[test]
fn a_floor_is_a_plane_or_it_is_nothing() {
    assert_eq!(Floor::none().level(), None);
    assert_eq!(Floor::default().level(), None, "a default scene has none");
    assert_eq!(Floor::at(GROUND).level(), Some(GROUND));
}

/// How hard a body's skin is taken to hold cloth, as the engine asks for it.
const SKIN: Grip = Grip::coulomb(0.6, 0.4);

/// One landing on the plane: where the particle ends up sideways, having come
/// from `q` and arrived `depth` metres under it after a `sideways` step.
fn lands(grip: Grip, depth: f32, sideways: f32) -> f32 {
    let q = [0.0, GROUND + 0.010, 0.0];
    let p = [sideways, GROUND - depth, 0.0];
    rest_on(GROUND, grip, p, q).0[0]
}

/// A slipping contact takes half the sideways motion and nothing else decides
/// it — not how hard the surface was pressed, and not how far the particle was
/// going.
///
/// This is the arithmetic every drape golden is hashed from, written down as a
/// number rather than left to be inferred from the constant: a contact that
/// began reading the push here would move all eight of them.
#[test]
fn a_slipping_contact_takes_half_the_motion_whatever_the_push_was() {
    for depth in [0.0001, 0.001, 0.010] {
        assert_eq!(lands(Grip::slipping(), depth, 0.020), 0.010, "{depth}");
        assert_eq!(lands(Grip::default(), depth, 0.020), 0.010, "{depth}");
    }
}

/// A contact that reads the push holds a slide it can and lets go of one it
/// cannot, and the line between them is the depth it just corrected.
///
/// Sideways by a millimetre: pressed four millimetres in, the static
/// coefficient's share of that is 2.4 mm and the whole of the motion goes, so
/// the particle ends where it began the substep. Pressed a fifth of a
/// millimetre in, the share is 0.12 mm and only the kinetic coefficient's part
/// of it is taken, which leaves the particle still travelling.
#[test]
fn a_contact_that_reads_the_push_sticks_when_it_can_and_slides_when_it_cannot() {
    let held = lands(SKIN, 0.004, 0.001);
    assert_eq!(held, 0.0, "it did not move at all: {held}");

    let slid = lands(SKIN, 0.0002, 0.001);
    let taken = 0.4 * 0.0002;
    assert!(
        (slid - (0.001 - taken)).abs() < 1.0e-7,
        "the kinetic share of the depth, and no more: {slid}"
    );
}

/// And the push is what decides, which is the whole of what the fixed share
/// could not say: the same sideways step from two depths comes out at two
/// places, and the harder-pressed one is held better.
#[test]
fn the_harder_a_contact_is_pressed_the_less_of_the_slide_is_left() {
    let mut last = f32::MAX;
    for depth in [0.0001, 0.0010, 0.0050, 0.0200] {
        let left = lands(SKIN, depth, 0.010);
        assert!(left < last, "{depth} m deep left {left}, more than {last}");
        last = left;
    }
    assert_eq!(last, 0.0, "and deep enough, it holds outright");
    // The fixed share reads the same at every one of those depths, which is
    // why a waistband squeezing harder bought nothing under it.
    let fixed: Vec<f32> = [0.0001, 0.0200]
        .into_iter()
        .map(|depth| lands(Grip::slipping(), depth, 0.010))
        .collect();
    assert_eq!(fixed[0], fixed[1]);
}

/// The field's own contact reads the push the same way, and a slipping one
/// there still takes the share it always took.
#[test]
fn the_body_holds_what_it_presses_and_slips_what_it_does_not() {
    let sdf = saturated();
    let q = [0.0250, 0.0140, 0.015];
    let p = [0.0245, 0.0150, 0.015];
    let depth = -sdf.sample(p[0], p[1], p[2]);
    assert!(depth > 0.0, "it is under the skin by {depth}");
    let slipping = resolve(&sdf, EPS, Grip::slipping(), p, q).0;
    let gripping = resolve(&sdf, EPS, SKIN, p, q).0;
    // The push is along x here, so the sideways motion is the y step, and the
    // static share of a push that deep covers the whole of it.
    assert!(
        (gripping[1] - q[1]).abs() < 1.0e-9,
        "the skin held it: {gripping:?} against {q:?}"
    );
    assert!(
        (slipping[1] - q[1]).abs() > 1.0e-6,
        "and the fixed share only ever takes half: {slipping:?}"
    );
}
