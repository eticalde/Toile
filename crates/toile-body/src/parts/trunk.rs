use super::{Ctx, Part, head, tags};
use crate::loft::loft;
use crate::profile::{dome, tube};
use crate::ring::{Ring, Shape, from_width};
use crate::station::{Station, TRUNK};

// Section shapes of the torso, crotch to shoulders. Each ring's centre stays
// on the side-seam plane, so the spine's S-curve and the bust's lead emerge
// from the front/back depth differences alone: a flat, squarer back at the
// waist, a full seat behind the hip, blades behind the armpit.
const CROTCH: Shape = Shape {
    rho_front: 0.62,
    rho_back: 0.82,
    round_front: 0.60,
    round_back: 0.60,
};
const WAIST: Shape = Shape {
    rho_front: 0.76,
    rho_back: 0.66,
    round_front: 0.55,
    round_back: 0.85,
};
const UNDERBUST: Shape = Shape {
    rho_front: 0.74,
    rho_back: 0.70,
    round_front: 0.50,
    round_back: 0.75,
};
const ARMPIT: Shape = Shape {
    rho_front: 0.66,
    rho_back: 0.76,
    round_front: 0.55,
    round_back: 0.80,
};
const SHOULDER: Shape = Shape {
    rho_front: 0.34,
    rho_back: 0.44,
    round_front: 0.70,
    round_back: 0.70,
};

/// The hip ring: the seat deepens with the hip-to-waist difference, so a
/// 30 cm drop stands further behind the waist than a 10 cm one.
fn hip_shape(hip: f64, waist: f64) -> Shape {
    Shape {
        rho_front: 0.66,
        rho_back: 0.84 + 0.003 * (hip - waist - 10.0).clamp(0.0, 40.0),
        round_front: 0.55,
        round_back: 0.45,
    }
}

/// The bust ring: the bust-over-underbust difference (clamped to 25 cm) pushes
/// the front depth out and rounds the front off, which is how a bust girth
/// above the underbust produces forward prominence with no sex switch.
fn bust_shape(bust: f64, underbust: f64) -> Shape {
    let delta = (bust - underbust).clamp(0.0, 25.0);
    Shape {
        rho_front: 0.70 + 0.02 * delta,
        rho_back: 0.72,
        round_front: (0.55 - 0.02 * delta).clamp(0.15, 0.55),
        round_back: 0.80,
    }
}

/// The half-width of the shoulder ring: half the across-shoulder width, but
/// never narrower than the neck it has to carry.
pub(crate) fn shoulder_half_width(c: &Ctx) -> f64 {
    let a_neck = head::neck_base(c).a;
    (0.5 * c.m.shoulder_width * 0.01).max(1.2 * a_neck)
}

/// The hip station, which the crotch and the legs are sized from.
pub(crate) fn hip_ring(c: &Ctx) -> Ring {
    c.girth_ring(c.m.hip, hip_shape(c.m.hip, c.m.waist), 0.0, c.lm.hip)
}

/// The twelve trunk stations, crotch to the head's widest ring: 0 crotch,
/// 1 hip, 2 waist, 3 underbust, 4 bust, 5 armpit, 6 shoulder, then the five
/// neck and head stations.
pub(crate) fn stations(c: &Ctx) -> [Ring; 12] {
    let m = c.m;
    let lm = c.lm;
    let hip = hip_ring(c);
    // The crotch narrows a hair under the hip; its width, not a tape, is the
    // measure because no tape runs there.
    let crotch = from_width(0.98 * hip.a, CROTCH, 0.0, 0.0, lm.crotch);
    let shoulder = from_width(shoulder_half_width(c), SHOULDER, 0.0, 0.0, lm.shoulder);
    let [neck_base, neck_top, jaw, cheek, head_max] = head::stations(c);
    [
        crotch,
        hip,
        c.girth_ring(m.waist, WAIST, 0.0, lm.waist),
        c.girth_ring(m.underbust, UNDERBUST, 0.0, lm.underbust),
        c.girth_ring(m.bust, bust_shape(m.bust, m.underbust), 0.0, lm.bust),
        c.girth_ring(m.upper_chest, ARMPIT, 0.0, lm.armpit),
        shoulder,
        neck_base,
        neck_top,
        jaw,
        cheek,
        head_max,
    ]
}

/// The trunk, neck and head as one loft: crotch to the head's widest ring
/// through the stations, then the skull cap to the crown. One tube means no
/// shading seam at the neck and one ring count. The bottom is capped (hidden
/// between the legs); the apex ring closes itself.
pub(crate) fn trunk(c: &Ctx) -> Part {
    let secs = stations(c);
    let mut rings = tube(&secs, c.res.loft_steps, c.dirs);
    let mut marks = tags(&TRUNK, c.res.loft_steps);
    rings.extend(dome(&secs[11], c.lm.crown, c.res.dome_rings, c.dirs));
    marks.extend(std::iter::repeat_n(
        Station::Crown.tag(),
        c.res.dome_rings as usize,
    ));
    loft(&rings, &marks, true, false)
}
