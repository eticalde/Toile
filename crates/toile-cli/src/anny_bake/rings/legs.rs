use super::geom::{add, lerp, sub, unit};
use super::intersect::Crossing;
use super::{Joints, intersect, limb_ring, select};

/// How far along the thigh axis (from the hip joint toward the knee)
/// `muslo` is cut. Below about a fifth of the way, the mesh has not yet
/// separated the two legs from the pelvis; a third of the way is safely
/// past that and still "near the top of the thigh."
const THIGH_T: f64 = 0.30;

/// How far along the lower-leg axis (from the knee toward the ankle joint)
/// `tobillo` is cut — see `crate::anny_bake`'s doc on the departure.
const ANKLE_T: f64 = 0.85;

/// The neck and leg rings: the ones cut on a single limb's own axis with no
/// band search, since none of them sit near a fused, ambiguous section.
pub(super) struct LegAndNeckRings {
    pub neck: Vec<Crossing>,
    pub thigh: Vec<Crossing>,
    pub knee: Vec<Crossing>,
    pub ankle: Vec<Crossing>,
}

pub(super) fn bake(positions: &[[f64; 3]], tris: &[[u32; 3]], j: &Joints) -> LegAndNeckRings {
    let found = intersect::loops(positions, tris, j.neck, sub(j.head, j.neck));
    let neck = select::pick_nearest_loop(positions, &found, j.neck)
        .expect("neck: no cross-section found at the neck joint");

    let thigh_point = lerp(j.upper_leg, j.knee, THIGH_T);
    let thigh = limb_ring(
        positions,
        tris,
        thigh_point,
        sub(j.knee, j.upper_leg),
        "muslo",
    );

    let knee_axis = add(unit(sub(j.knee, j.upper_leg)), unit(sub(j.ankle, j.knee)));
    let knee = limb_ring(positions, tris, j.knee, knee_axis, "rodilla");

    let ankle_point = lerp(j.knee, j.ankle, ANKLE_T);
    let ankle = limb_ring(
        positions,
        tris,
        ankle_point,
        sub(j.ankle, j.knee),
        "tobillo",
    );

    LegAndNeckRings {
        neck,
        thigh,
        knee,
        ankle,
    }
}
