use super::geom::{add, lerp, nearest_vertex, sub, topmost_vertex_within, unit};
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
/// gender-differentiated girth. `RingId::WristJoint` — a different
/// landmark, at the literal hand joint — is what `brazo`'s length actually
/// ends at; see this module's `bake` doc.
const WRIST_T: f64 = 0.75;

/// How far from `joint-r-shoulder` the acromion search looks: generous
/// enough to cover the deltoid cap over the joint, tight enough that it
/// cannot wander onto the chest or the neck.
const ACROMION_SEARCH_RADIUS_M: f64 = 0.07;

/// The arm rings: the upper-arm girth and its shared offset, the two
/// shoulder landmarks, the elbow landmark, the wrist girth, and the two
/// single-point landmarks `brazo`'s length is actually measured between.
pub(super) struct ArmRings {
    pub upper_arm: Vec<Crossing>,
    pub shoulder_r: Vec<Crossing>,
    pub shoulder_l: Vec<Crossing>,
    pub elbow: Vec<Crossing>,
    pub wrist: Vec<Crossing>,
    pub acromion: Vec<Crossing>,
    pub wrist_joint: Vec<Crossing>,
}

/// A girth ring's own position and a length's end point do not have to be
/// the same vertex — see `crate::anny_bake`'s doc. `upper_arm`, `shoulder_r`
/// and `shoulder_l` sit where the plane can actually separate arm from
/// torso (well below the true shoulder joint); `wrist` sits three-quarters
/// down the forearm, where the girth actually responds to the morphs. But
/// `brazo` — a *length* — is free to run between the real joints: the
/// acromion (the shoulder's own bony top, found on the mesh surface rather
/// than at the ball joint `joint-r-shoulder` marks) and the wrist joint
/// (the nearest vertex to `joint-r-hand`, not the girth ring's own offset
/// point).
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

    let acromion_vertex = topmost_vertex_within(positions, j.shoulder_r, ACROMION_SEARCH_RADIUS_M);
    let acromion = vec![(acromion_vertex, acromion_vertex, 0.0)];

    let wrist_vertex = nearest_vertex(positions, j.hand_r);
    let wrist_joint = vec![(wrist_vertex, wrist_vertex, 0.0)];

    ArmRings {
        upper_arm,
        shoulder_r,
        shoulder_l,
        elbow,
        wrist,
        acromion,
        wrist_joint,
    }
}
