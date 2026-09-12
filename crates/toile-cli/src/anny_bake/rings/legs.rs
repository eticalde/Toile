use super::geom::{add, lerp, most_posterior_in_band, nearest_vertex, sub, unit};
use super::{Cut, Joints, intersect, limb_ring, select};

/// How far along the thigh axis (from the hip joint toward the knee)
/// `muslo` is cut. Below about a fifth of the way, the mesh has not yet
/// separated the two legs from the pelvis; a third of the way is safely
/// past that and still "near the top of the thigh."
const THIGH_T: f64 = 0.30;

/// How far along the lower-leg axis (from the knee toward the ankle joint)
/// `tobillo` is cut — see `crate::anny_bake`'s doc on the departure.
const ANKLE_T: f64 = 0.85;

/// How far the nape search reaches from the sagittal midline: tight enough
/// that it cannot wander onto a shoulder blade, generous enough to find the
/// C7 vertebra's own bump even though it is not perfectly centred.
const NAPE_MIDLINE_TOLERANCE_M: f64 = 0.03;

/// The height band the nape search covers, as a margin above and below the
/// neck joint. Chosen empirically, not merely assumed: searching lower,
/// down toward the shoulder joint, sounds like the anatomically obvious
/// direction for "the base of the neck," but on this mesh it lands closer
/// to the waist and *shortens* `largo_espalda` instead of lengthening it.
/// A margin centred on the neck joint itself is both the anatomically
/// sound reading (this rig's own "neck" joint already sits at the base of
/// the neck, not mid-neck — see `crate::anny_bake`'s doc) and the
/// empirically better of the two.
const NAPE_BAND_MARGIN_M: f64 = 0.02;

/// The neck and leg rings: the ones cut on a single limb's own axis with no
/// band search, since none of them sit near a fused, ambiguous section. Plus
/// `ankle_joint` and `nape`, single-point landmarks rather than cuts — see
/// [`bake`]'s doc.
pub(super) struct LegAndNeckRings {
    pub neck: Cut,
    pub thigh: Cut,
    pub knee: Cut,
    pub ankle: Cut,
    pub ankle_joint: Cut,
    pub nape: Cut,
}

/// `neck` is cut at the neck joint — right for `cuello`'s collar height —
/// but a ring cut there is a full loop around the neck, so its own most
/// posterior point still sits partway around that loop rather than
/// squarely on the spine. `nape` is found separately: the most posterior
/// vertex in a band centred on the neck joint's own height, near the
/// midline — a genuine surface search rather than a point already fixed
/// by the neck ring's own topology.
///
/// `ankle_joint` is the same kind of point at the other end of the body,
/// and it is not `ankle`: the girth ring sits at the narrowest section the
/// shin offers, 12.9 cm above the sole on this template, while the ankle
/// bone a leg length stops at is at 6.8 cm — [`nearest_vertex`] lands on the
/// medial malleolus there, inside the 6–8 cm band a tailor's tape ends in.
///
/// The gap between the two is the mesh's, not a placement mistake. Scanning
/// the lower leg perpendicular to its own axis, the section shrinks
/// monotonically from the knee to a 20.5 cm minimum at that 12.9 cm, then
/// grows again all the way down: 21.1 at 11.0 cm, 23.8 at 9.1, 31.5 at 6.9.
/// So there is no second, lower narrowing over the malleoli for a "narrowest
/// section above the foot" rule to find, and it is the foot's own flare
/// arriving early that swallows it — over that same descent the section's
/// front-to-back extent runs 7.4 → 9.0 → 12.7 cm while its left-right extent
/// only reaches 5.6 → 6.3 → 7.0, which is the heel entering the loop rather
/// than the ankle widening.
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

    let ankle_joint = Cut::landmark(nearest_vertex(positions, j.ankle));

    let nape = Cut::landmark(most_posterior_in_band(
        positions,
        j.neck[1] - NAPE_BAND_MARGIN_M,
        j.neck[1] + NAPE_BAND_MARGIN_M,
        NAPE_MIDLINE_TOLERANCE_M,
    ));

    LegAndNeckRings {
        neck,
        thigh,
        knee,
        ankle,
        ankle_joint,
        nape,
    }
}
