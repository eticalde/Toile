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

/// Deep in the saturated interior there is no normal, and a particle there is
/// left exactly where it is — with the motion it arrived carrying.
///
/// The push along a zero gradient moves nothing, so the friction and the
/// damping that follow would be spent against a normal that does not exist:
/// they would take away every bit of tangential motion and stop the particle
/// dead. The stretch constraints are what draw such a particle out, through
/// the neighbours that do have a gradient, and they cannot do it if the
/// contact solve has already frozen it.
#[test]
fn a_particle_in_the_saturated_interior_is_left_to_the_constraints() {
    let sdf = saturated();
    let p = [0.015, 0.015, 0.015];
    assert_eq!(sdf.sample(p[0], p[1], p[2]), -BAND, "it is under the skin");
    let q = [0.010, 0.015, 0.015];
    let (moved, before) = resolve(&sdf, EPS, p, q);
    assert_eq!(moved, p, "nothing pushed it, so nothing may move it");
    assert_eq!(before, q, "and the motion it had is still there to be read");
}

/// A particle the band does reach is still carried out of the body, which is
/// the case the one above must not have broken.
#[test]
fn a_particle_the_band_reaches_is_still_pushed_out() {
    let sdf = saturated();
    let p = [0.025, 0.015, 0.015];
    assert!(sdf.sample(p[0], p[1], p[2]) < 0.0, "it is under the skin");
    let (moved, _) = resolve(&sdf, EPS, p, p);
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
    let (moved, before) = rest_on(GROUND, p, q);
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
    let (moved, before) = rest_on(GROUND, p, q);
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
