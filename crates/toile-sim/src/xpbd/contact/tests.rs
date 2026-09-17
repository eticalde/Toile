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
