use super::geom::lerp;
use super::{Cut, Joints, bands, limb_ring};

/// How far along the forearm axis (from the elbow toward the hand joint)
/// `muneca` is cut. Not at the hand joint itself (`t = 1.0`): the forearm
/// tapers smoothly all the way into the hand with no distinct wrist-bone
/// pinch on this mesh, so a cut right at the joint sits deep enough into
/// the hand/wrist boundary that it barely responds to the build and gender
/// morphs at all (male and female landed within 3 mm of each other there).
/// Backing off to three-quarters of the way down the forearm still reads
/// as "the wrist," not "the forearm," and recovers a normal-sized,
/// gender-differentiated girth. `brazo`, a length, is free to run to the
/// hand joint itself; see `paths::arm`.
const WRIST_T: f64 = 0.75;

/// The arm's two girth rings.
pub(super) struct ArmRings {
    pub upper_arm: Cut,
    pub wrist: Cut,
}

/// `brazo_contorno` at the fullest section [`bands::limb_fullest`] finds
/// where a plane can still separate arm from torso, well below the true
/// shoulder joint; `muneca` three-quarters down the forearm, where the girth
/// responds to the morphs.
pub(super) fn bake(positions: &[[f64; 3]], tris: &[[u32; 3]], j: &Joints) -> ArmRings {
    let upper_arm = bands::limb_fullest(positions, tris, j.shoulder_r, j.elbow_r);
    let wrist_point = lerp(j.elbow_r, j.hand_r, WRIST_T);
    let wrist = limb_ring(
        positions,
        tris,
        wrist_point,
        super::geom::sub(j.hand_r, j.elbow_r),
        "muneca",
    );
    ArmRings { upper_arm, wrist }
}
