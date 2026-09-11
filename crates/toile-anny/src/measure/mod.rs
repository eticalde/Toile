use crate::asset::RingId;
use crate::mesh::decoded;

mod geom;
#[cfg(test)]
mod tests;

use geom::{centroid, dist, perimeter};

/// The number of body vertices every baked ring indexes into (see `crate`'s
/// doc): the fixed shape [`measure`] requires of its `positions` argument.
const BODY_VERTEX_COUNT: usize = 13_380;

/// The 20 catalogue measurements read directly off a generated Anny mesh, in
/// centimetres.
///
/// Field names mirror `toile_body::PartialMeasures`'s English identifiers —
/// the Spanish catalogue names those map to are the boundary layer's
/// business (`toile_engine::body`), never this crate's, per STD-001.
///
/// This is the *measured* half of the tab's dado/medido/Δ row: what this
/// body actually comes to, as opposed to what the tape (a [`crate::Phenotype`]
/// does not itself carry a tape) was written down as.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Measures {
    /// The mesh's own bounding-box height (`estatura`).
    pub height: f32,
    /// The neck ring's perimeter (`cuello`).
    pub neck: f32,
    /// The bust ring's perimeter (`pecho`).
    pub bust: f32,
    /// The upper-chest ring's perimeter (`pecho_alto`).
    pub upper_chest: f32,
    /// The underbust ring's perimeter (`bajo_pecho`).
    pub underbust: f32,
    /// The waist ring's perimeter (`cintura`).
    pub waist: f32,
    /// The hip ring's perimeter (`cadera`).
    pub hip: f32,
    /// The thigh ring's perimeter (`muslo`).
    pub thigh: f32,
    /// The knee ring's perimeter (`rodilla`).
    pub knee: f32,
    /// The ankle ring's perimeter (`tobillo`).
    pub ankle: f32,
    /// The upper-arm ring's perimeter (`brazo_contorno`).
    pub upper_arm: f32,
    /// The wrist ring's perimeter (`muneca`).
    pub wrist: f32,
    /// The head ring's perimeter (`cabeza`).
    pub head: f32,
    /// Waist centroid to crotch centroid (`tiro`).
    pub rise: f32,
    /// Waist centroid to hip centroid (`altura_cadera`).
    pub hip_drop: f32,
    /// Crotch centroid to ankle centroid (`entrepierna`).
    pub inseam: f32,
    /// Waist centroid to ankle centroid (`largo_lateral`); a straight
    /// distance, not the true path down the outside of the leg ISO 8559
    /// specifies — see [`measure`]'s doc.
    pub outseam: f32,
    /// The nape landmark ([`RingId::NapeBase`]) to waist centroid
    /// (`largo_espalda`) — see [`measure`]'s doc on why the nape sits below
    /// the neck ring, not on it.
    pub back_length: f32,
    /// The acromion ([`RingId::Acromion`], not the [`RingId::ShoulderRight`]
    /// girth ring, which cannot sit at the true shoulder) to elbow, plus
    /// elbow to the wrist joint ([`RingId::WristJoint`], not the
    /// [`RingId::Wrist`] girth ring), summed along the two segments rather
    /// than measured straight (`brazo`).
    pub arm_length: f32,
    /// Left shoulder centroid to right shoulder centroid (`hombros`).
    pub shoulder_width: f32,
}

/// The ring a [`RingId`] names, as a slice of the shipped asset's flat
/// point array.
fn ring_points(id: RingId) -> &'static [crate::asset::RingPoint] {
    let baked = decoded();
    let r = baked.ring_ranges[id as usize];
    let start = r.offset as usize;
    &baked.ring_points[start..start + r.length as usize]
}

/// Measures every catalogue value off `positions` (already morphed by a
/// phenotype), by walking the nineteen rings baked into the shipped asset.
///
/// See `crate`'s module doc for why the rings are cut once at bake time
/// rather than intersected against `positions` here. Every step is
/// `+ - * / sqrt` over the ring's fixed `(vertex, vertex, t)`
/// data and the caller's positions, in the fixed order above: the same
/// regime [`crate::body_mesh`] itself runs in, so a girth is a continuous,
/// differentiable function of the phenotype — the property the next
/// slice's per-part solver depends on.
///
/// `largo_lateral` (`outseam`) is reported as the straight centroid-to-
/// centroid distance rather than the true path down the outside of the leg
/// ISO 8559 specifies; that is an honest approximation, not a bug, and the
/// interface should say so rather than claim ISO conformance. The hip-to-
/// waist relationship is structural rather than incidental: [`RingId::Hip`]
/// is baked strictly above [`RingId::Crotch`] (the fork), so `hip_drop`
/// (`altura_cadera`) is always shorter than `rise` (`tiro`) — see the
/// crate's tests.
///
/// `back_length` (nape to waist) is the one measurement still reported
/// short of a generic anthropometric table after an honest search for a
/// better landmark: it lands around 32–36 cm on the reference bodies
/// against a generic ~47–52 cm table value. [`RingId::NapeBase`] is found
/// by direct surface search rather than assumed, and searching lower —
/// down toward the shoulder, the anatomically obvious direction for "the
/// base of the neck" — measurably made it worse, not better (see
/// `crate::anny_bake`'s doc for the evidence); reaching 47–52 cm this way
/// would require a "nape" already up in jaw or skull territory. This is
/// this specific mesh's own proportion — a short neck-to-waist span — not
/// a ring left in the wrong place, and [`RingId::Waist`] stays exactly
/// where it is regardless: its girth matches independent `PyTorch`
/// measurements of this same mesh, so moving it to lengthen `back_length`
/// would trade one honest number for another.
///
/// # Panics
/// If `positions` does not hold exactly [`BODY_VERTEX_COUNT`] xyz triples.
/// The baked rings index into that exact vertex layout — the Anny body's,
/// never the tailor's dummy's — so a mismatched slice would otherwise read
/// out of bounds or silently measure the wrong vertices.
pub fn measure(positions: &[f32]) -> Measures {
    assert_eq!(
        positions.len(),
        BODY_VERTEX_COUNT * 3,
        "measure() only understands the Anny body's own {BODY_VERTEX_COUNT}-vertex layout, \
         got {} vertices",
        positions.len() / 3
    );

    let per = |id: RingId| perimeter(positions, ring_points(id)) * 100.0;
    let cen = |id: RingId| centroid(positions, ring_points(id));
    let cm = |a: [f32; 3], b: [f32; 3]| dist(a, b) * 100.0;

    let ys = positions.iter().skip(1).step_by(3);
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for &y in ys {
        lo = lo.min(y);
        hi = hi.max(y);
    }
    let height = (hi - lo) * 100.0;

    let waist_c = cen(RingId::Waist);
    let hip_c = cen(RingId::Hip);
    let crotch_c = cen(RingId::Crotch);
    let ankle_c = cen(RingId::Ankle);
    let nape_c = cen(RingId::NapeBase);
    let shoulder_l_c = cen(RingId::ShoulderLeft);
    let shoulder_r_c = cen(RingId::ShoulderRight);
    let acromion_c = cen(RingId::Acromion);
    let elbow_c = cen(RingId::Elbow);
    let wrist_joint_c = cen(RingId::WristJoint);

    Measures {
        height,
        neck: per(RingId::Neck),
        bust: per(RingId::Bust),
        upper_chest: per(RingId::UpperChest),
        underbust: per(RingId::Underbust),
        waist: per(RingId::Waist),
        hip: per(RingId::Hip),
        thigh: per(RingId::Thigh),
        knee: per(RingId::Knee),
        ankle: per(RingId::Ankle),
        upper_arm: per(RingId::UpperArm),
        wrist: per(RingId::Wrist),
        head: per(RingId::Head),
        rise: cm(waist_c, crotch_c),
        hip_drop: cm(waist_c, hip_c),
        inseam: cm(crotch_c, ankle_c),
        outseam: cm(waist_c, ankle_c),
        back_length: cm(nape_c, waist_c),
        arm_length: cm(acromion_c, elbow_c) + cm(elbow_c, wrist_joint_c),
        shoulder_width: cm(shoulder_l_c, shoulder_r_c),
    }
}
