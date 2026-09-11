use super::geom::{add, lerp, nearest_vertex, sub, unit};
use super::intersect::Crossing;
use super::{Joints, bands, limb_ring};

/// How far along the forearm axis (from the elbow toward the hand joint)
/// `muneca` is cut. Not at the hand joint itself (`t = 1.0`): the forearm
/// tapers smoothly all the way into the hand with no distinct wrist-bone
/// pinch on this mesh, so a cut right at the joint sits deep enough into
/// the hand/wrist boundary that it barely responds to the build and gender
/// morphs at all (male and female landed within 3 mm of each other there).
/// Backing off to three-quarters of the way down the forearm still reads
/// as "the wrist," not "the forearm," and recovers a normal-sized,
/// gender-differentiated girth.
const WRIST_T: f64 = 0.75;

/// The arm rings: the upper-arm girth and its shared offset, the two
/// shoulder landmarks, the elbow landmark, the wrist, and the single-point
/// shoulder-joint landmark `brazo` starts from.
pub(super) struct ArmRings {
    pub upper_arm: Vec<Crossing>,
    pub shoulder_r: Vec<Crossing>,
    pub shoulder_l: Vec<Crossing>,
    pub elbow: Vec<Crossing>,
    pub wrist: Vec<Crossing>,
    pub shoulder_joint: Vec<Crossing>,
}

pub(super) fn bake(positions: &[[f64; 3]], tris: &[[u32; 3]], j: &Joints) -> ArmRings {
    let (upper_arm, shoulder_t) = bands::limb_fullest(positions, tris, j.shoulder_r, j.elbow_r);

    let shoulder_r_point = lerp(j.shoulder_r, j.elbow_r, shoulder_t);
    let shoulder_r = limb_ring(
        positions,
        tris,
        shoulder_r_point,
        sub(j.elbow_r, j.shoulder_r),
        "shoulder (right)",
    );
    let shoulder_l_point = lerp(j.shoulder_l, j.elbow_l, shoulder_t);
    let shoulder_l = limb_ring(
        positions,
        tris,
        shoulder_l_point,
        sub(j.elbow_l, j.shoulder_l),
        "shoulder (left)",
    );

    let elbow_axis = add(
        unit(sub(j.elbow_r, j.shoulder_r)),
        unit(sub(j.hand_r, j.elbow_r)),
    );
    let elbow = limb_ring(positions, tris, j.elbow_r, elbow_axis, "elbow");

    let wrist_point = lerp(j.elbow_r, j.hand_r, WRIST_T);
    let wrist = limb_ring(
        positions,
        tris,
        wrist_point,
        sub(j.hand_r, j.elbow_r),
        "muneca",
    );

    // Not a cut: the plane at the true shoulder joint cannot separate arm
    // from torso at all (see `crate::anny_bake`'s doc), so `brazo`'s start
    // point is the single nearest body vertex instead of a ring.
    let shoulder_vertex = nearest_vertex(positions, j.shoulder_r);
    let shoulder_joint = vec![(shoulder_vertex, shoulder_vertex, 0.0)];

    ArmRings {
        upper_arm,
        shoulder_r,
        shoulder_l,
        elbow,
        wrist,
        shoulder_joint,
    }
}
