use super::{Ctx, Part, Side, place, shoulder_half_width, trunk};
use crate::loft::loft;
use crate::profile::tube;
use crate::ring::{Ring, Shape, from_girth};

/// The A-pose: each arm tilts 20° outward from the vertical. Pasted as
/// `cos`/`sin` literals because no trig may run at mesh time.
pub(crate) const POSE_COS: f64 = 0.939_692_620_785_908_4;
pub(crate) const POSE_SIN: f64 = 0.342_020_143_325_668_7;

/// Per-side arm carriage: the joint hangs a hair ahead of the side seam.
const JOINT_Z: f64 = 0.01;

// Leg sections, ankle to thigh: a calf that swells behind, a squarer knee.
const ANKLE: Shape = Shape {
    rho_front: 1.00,
    rho_back: 1.05,
    round_front: 0.15,
    round_back: 0.15,
};
const CALF: Shape = Shape {
    rho_front: 0.95,
    rho_back: 1.15,
    round_front: 0.10,
    round_back: 0.10,
};
const KNEE: Shape = Shape {
    rho_front: 1.00,
    rho_back: 1.00,
    round_front: 0.25,
    round_back: 0.25,
};
const THIGH: Shape = Shape {
    rho_front: 1.10,
    rho_back: 1.10,
    round_front: 0.10,
    round_back: 0.10,
};

// Arm sections, wrist to deltoid, in the arm's local frame.
const WRIST: Shape = Shape {
    rho_front: 1.15,
    rho_back: 1.15,
    round_front: 0.40,
    round_back: 0.40,
};
const FOREARM: Shape = Shape {
    rho_front: 0.95,
    rho_back: 1.05,
    round_front: 0.10,
    round_back: 0.10,
};
const ELBOW: Shape = Shape {
    rho_front: 1.00,
    rho_back: 1.00,
    round_front: 0.30,
    round_back: 0.30,
};
const BICEPS: Shape = Shape {
    rho_front: 1.05,
    rho_back: 0.95,
    round_front: 0.10,
    round_back: 0.10,
};
const DELTOID: Shape = Shape {
    rho_front: 1.00,
    rho_back: 1.00,
    round_front: 0.10,
    round_back: 0.10,
};

fn thigh_ring(c: &Ctx, cx: f64) -> Ring {
    // The thigh top sits just above the crotch, inside the trunk: a hidden
    // overlap a dummy accepts (a watertight pelvis merge is a follow-up).
    from_girth(c.m.thigh, THIGH, cx, 0.0, c.lm.crotch + 0.03, c.dirs)
}

/// The leg spacing from the body axis: the thigh's outer edge tucks just
/// inside the pelvis while the two thighs touch at the inseam.
pub(crate) fn leg_offset(c: &Ctx) -> f64 {
    let a_hip = trunk::hip_ring(c).a;
    let a_thigh = thigh_ring(c, 0.0).a;
    (a_hip - a_thigh - 0.005).clamp(0.25 * a_hip, 0.5 * a_hip)
}

/// One leg, vertical, capped at both ends: 0 ankle, 1 calf, 2 knee, 3 thigh.
pub(crate) fn leg(c: &Ctx, side: Side) -> Part {
    let cx = side.sign() * leg_offset(c);
    let m = c.m;
    let secs = [
        from_girth(m.ankle, ANKLE, cx, 0.0, 0.0, c.dirs),
        // No tape for the calf yet; it hangs on the thigh until the catalogue
        // names it.
        from_girth(0.63 * m.thigh, CALF, cx, 0.0, c.lm.calf, c.dirs),
        from_girth(m.knee, KNEE, cx, 0.0, c.lm.knee, c.dirs),
        thigh_ring(c, cx),
    ];
    loft(&tube(&secs, c.res.loft_steps, c.dirs), true, true)
}

/// The arm's length from the joint, kept away from zero so its stations stay
/// distinct whatever the tape says.
fn arm_span(c: &Ctx) -> f64 {
    (c.m.arm_length * 0.01).max(0.05)
}

/// The arm stations in the local frame, wrist (`y = −length`) to deltoid
/// (`y = 0`): 0 wrist, 1 forearm, 2 elbow, 3 biceps, 4 deltoid.
fn arm_stations(c: &Ctx) -> [Ring; 5] {
    let len = arm_span(c);
    let ua = c.m.upper_arm;
    let local = |girth: f64, s: Shape, y: f64| from_girth(girth, s, 0.0, 0.0, y, c.dirs);
    [
        local(c.m.wrist, WRIST, -len),
        local(0.90 * ua, FOREARM, -0.68 * len),
        local(0.87 * ua, ELBOW, -0.56 * len),
        local(ua, BICEPS, -0.30 * len),
        local(1.05 * ua, DELTOID, 0.0),
    ]
}

/// The shoulder joint of one arm: inside the shoulder ring by the deltoid's
/// radius, a little under the acromion, so the arm's top cap hides in the
/// trunk the way the thigh top hides in the pelvis.
pub(crate) fn joint(c: &Ctx, side: Side) -> [f64; 3] {
    let r_top = arm_stations(c)[4].a;
    let a_sh = shoulder_half_width(c);
    let jx = (a_sh - r_top - 0.01).max(0.25 * a_sh);
    let jy = c.lm.shoulder - 0.6 * r_top;
    [side.sign() * jx, jy, JOINT_Z]
}

/// One arm in the A-pose, capped at both ends: the wrist cap shows, the joint
/// cap hides inside the shoulder.
pub(crate) fn arm(c: &Ctx, side: Side) -> Part {
    let mut rings = tube(&arm_stations(c), c.res.loft_steps, c.dirs);
    place(&mut rings, side.sign(), joint(c, side), POSE_COS, POSE_SIN);
    loft(&rings, true, true)
}
