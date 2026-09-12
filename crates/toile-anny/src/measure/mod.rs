use crate::asset::RingId;
use crate::mesh::decoded;

mod geom;
#[cfg(test)]
mod tests;

use geom::{centroid, dist, perimeter};

/// The number of body vertices every baked ring indexes into: the fixed
/// shape [`measure`] requires of its `positions` argument.
const BODY_VERTEX_COUNT: usize = 13_380;

/// The 20 catalogue measurements read directly off a generated Anny mesh, in
/// centimetres.
///
/// Field names are English identifiers — the Spanish catalogue names these
/// map to are the boundary layer's business (`toile_engine::body`), never
/// this crate's, per STD-001.
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
    ///
    /// This mesh's horizontal section shrinks monotonically from the armpit
    /// down to the waist, so a ring cut anywhere above `pecho`'s own
    /// necessarily reads *more* than `pecho`. A tape asking for less than
    /// its own `pecho` therefore cannot be met at any honest height, and
    /// this row has no lever of its own to close the rest.
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
    ///
    /// Always shorter than `rise`, structurally rather than by luck:
    /// [`RingId::Hip`] is baked strictly above [`RingId::Crotch`], so
    /// however a phenotype moves the three, the hip can never be as far
    /// from the waist as the crotch is.
    pub hip_drop: f32,
    /// The crotch centroid's height above the floor (`entrepierna`).
    ///
    /// A height rather than a straight line: the feet in Anny's A-pose
    /// stand well to either side of the midline the waist and crotch
    /// centroids lie on, so a straight line would fold the stance width
    /// into the reading and make it depend on the pose rather than on the
    /// body. Measured down to the same floor the stature is, so the parts
    /// still sum to the whole.
    pub inseam: f32,
    /// The waist centroid's height above the ankle landmark
    /// ([`RingId::AnkleJoint`], not the [`RingId::Ankle`] girth ring, which
    /// cannot sit at the ankle bone) — `largo_lateral`.
    ///
    /// A vertical drop for the same pose reason as `inseam`, but it stops
    /// at the ankle rather than the floor because the loft reads the ankle
    /// back out of it as `waist − largo_lateral`: measured to the floor
    /// that would put the ankle at zero and sink the foot caps below the
    /// ground. Still the waist's own drop rather than the path down the
    /// outside of the leg ISO 8559 specifies — an approximation the
    /// interface should state rather than claim conformance for.
    pub outseam: f32,
    /// The nape landmark ([`RingId::NapeBase`], below the neck ring rather
    /// than on it) to waist centroid (`largo_espalda`).
    ///
    /// Reads short of a generic anthropometric table on every reference
    /// body, and stays short at the best landmark a direct surface search
    /// finds: it is this mesh's own neck-to-waist proportion, not a
    /// landmark left in the wrong place.
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

/// The ring a [`RingId`] names: its points, as a slice of the shipped
/// asset's flat point array, and the plane the bake cut them on.
fn ring(id: RingId) -> (&'static [crate::asset::RingPoint], [f32; 3]) {
    let baked = decoded();
    let e = baked.ring_entries[id as usize];
    let start = e.offset as usize;
    (
        &baked.ring_points[start..start + e.length as usize],
        e.normal,
    )
}

/// Measures every catalogue value off `positions` (already morphed by a
/// phenotype), by walking the twenty rings baked into the shipped asset.
///
/// The rings are cut once at bake time and only ever walked here. Every
/// step is `+ - * / sqrt` over their fixed `(vertex, vertex, t)` data, in
/// the fixed order below — the same regime [`crate::body_mesh`] runs in, so
/// a girth is a continuous, differentiable function of the phenotype rather
/// than a step function of which triangle a plane happened to cross.
///
/// Every girth is summed in the plane its ring was cut on rather than in
/// space: see [`geom::perimeter`] and [`crate::asset::RingEntry::normal`].
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

    let per = |id: RingId| {
        let (points, normal) = ring(id);
        perimeter(positions, points, normal) * 100.0
    };
    let cen = |id: RingId| centroid(positions, ring(id).0);
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
    let ankle_c = cen(RingId::AnkleJoint);
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
        inseam: (crotch_c[1] - lo) * 100.0,
        outseam: (waist_c[1] - ankle_c[1]) * 100.0,
        back_length: cm(nape_c, waist_c),
        arm_length: cm(acromion_c, elbow_c) + cm(elbow_c, wrist_joint_c),
        shoulder_width: cm(shoulder_l_c, shoulder_r_c),
    }
}
