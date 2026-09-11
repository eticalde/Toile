use super::geom::{add, lerp, most_posterior_in_band, sub, unit};
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
/// `nape`, a single-point landmark rather than a cut — see [`bake`]'s doc.
pub(super) struct LegAndNeckRings {
    pub neck: Vec<Crossing>,
    pub thigh: Vec<Crossing>,
    pub knee: Vec<Crossing>,
    pub ankle: Vec<Crossing>,
    pub nape: Vec<Crossing>,
}

/// `neck` is cut at the neck joint — right for `cuello`'s collar height —
/// but a ring cut there is a full loop around the neck, so its own most
/// posterior point still sits partway around that loop rather than
/// squarely on the spine. `nape` is found separately: the most posterior
/// vertex in a band centred on the neck joint's own height, near the
/// midline — a genuine surface search rather than a point already fixed
/// by the neck ring's own topology.
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

    let nape_vertex = most_posterior_in_band(
        positions,
        j.neck[1] - NAPE_BAND_MARGIN_M,
        j.neck[1] + NAPE_BAND_MARGIN_M,
        NAPE_MIDLINE_TOLERANCE_M,
    );
    let nape = vec![(nape_vertex, nape_vertex, 0.0)];

    LegAndNeckRings {
        neck,
        thigh,
        knee,
        ankle,
        nape,
    }
}
