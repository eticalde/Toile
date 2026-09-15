use super::geom::{add, lerp, sub, unit};
use super::{Cut, Joints, intersect, limb_ring, select};

/// How far along the thigh axis (from the hip joint toward the knee)
/// `muslo` is cut. Below about a fifth of the way, the mesh has not yet
/// separated the two legs from the pelvis; a third of the way is safely
/// past that and still "near the top of the thigh."
const THIGH_T: f64 = 0.30;

/// How far along the lower-leg axis (from the knee toward the ankle joint)
/// `tobillo` is cut. Not at the ankle joint: that joint sits at the
/// boundary into the foot, where the section is already an elongated foot
/// shape rather than a round ankle. See [`bake`]'s doc for the scan that
/// fixes it here.
const ANKLE_T: f64 = 0.85;

/// The neck and leg rings: the ones cut on a single limb's own axis with no
/// band search, since none of them sit near a fused, ambiguous section.
pub(super) struct LegAndNeckRings {
    pub neck: Cut,
    pub thigh: Cut,
    pub knee: Cut,
    pub ankle: Cut,
}

/// The `ankle` ring lands well above the ankle bone the leg lengths stop at,
/// and that gap is this mesh's own, not a placement mistake: the ring lands
/// 12.9 cm above the sole on this template while the ankle bone is 6.8 cm
/// up. The scan that settles [`ANKLE_T`] is the evidence. Perpendicular to the
/// lower leg's axis, the section shrinks monotonically from the knee to a 20.5
/// cm minimum at that same 12.9 cm, then grows again all the way down: 21.1 at
/// 11.0, 23.8 at 9.1, 31.5 at 6.9. There is no second, lower narrowing over
/// the malleoli for a "narrowest section above the foot" rule to find,
/// because the foot's own flare arrives before them: over that same descent
/// the section's front-to-back extent runs 7.4 → 9.0 → 12.7 cm while its
/// left-right extent only reaches 5.6 → 6.3 → 7.0, which is the heel
/// entering the loop rather than the ankle widening.
pub(super) fn bake(positions: &[[f64; 3]], tris: &[[u32; 3]], j: &Joints) -> LegAndNeckRings {
    let neck_axis = sub(j.head, j.neck);
    let found = intersect::loops(positions, tris, j.neck, neck_axis);
    let neck = Cut::along(
        select::pick_nearest_loop(positions, &found, j.neck)
            .expect("neck: no cross-section found at the neck joint"),
        neck_axis,
    );

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
