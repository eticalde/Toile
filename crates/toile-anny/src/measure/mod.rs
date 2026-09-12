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
    /// The crotch centroid's height above the floor (`entrepierna`) — a
    /// tailor's tape runs to the ground, not the ankle; see [`measure`]'s
    /// doc.
    pub inseam: f32,
    /// The waist centroid's height above the ankle landmark
    /// ([`RingId::AnkleJoint`], not the [`RingId::Ankle`] girth ring, which
    /// cannot sit at the ankle bone) — `largo_lateral`. Not the true path
    /// down the outside of the leg ISO 8559 specifies; see [`measure`]'s
    /// doc.
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
/// See `crate`'s module doc for why the rings are cut once at bake time
/// rather than intersected against `positions` here. Every step is
/// `+ - * / sqrt` over the ring's fixed `(vertex, vertex, t)`
/// data and the caller's positions, in the fixed order above: the same
/// regime [`crate::body_mesh`] itself runs in, so a girth is a continuous,
/// differentiable function of the phenotype — the property the next
/// slice's per-part solver depends on.
///
/// Every girth is summed in the plane its ring was cut on
/// ([`crate::asset::RingEntry::normal`]), never in space. A ring is exactly
/// coplanar on the template — it is where a plane crossed the mesh — but
/// each of its points is glued to its own mesh edge, so a morph buckles the
/// loop out of that plane and a chord walk through space adds the buckling
/// to the girth. On the solved default tape what the projection takes back
/// out comes to 3.19 cm on `brazo_contorno`, 2.09 cm on `pecho_alto`,
/// 1.74 cm on `pecho` and 1.02 cm on `cuello`.
///
/// A row with a lever hid that rather than reporting it: the solver simply
/// shrank the body until the inflated reading matched the tape, so the error
/// left the Δ column and went into the mesh instead. Only the lever-less rows
/// ever showed it, which is why it went unnoticed for three slices.
///
/// The plane a ring is projected onto is the one it was *cut* on, which the
/// trunk keeps under a morph and a limb does not: the arm rings tilt about
/// 23° out of theirs on the solved body, so `brazo_contorno` is read across a
/// slightly oblique section and comes out around a centimetre under what a
/// tape lying flat on the arm would find. Still far nearer than the chord
/// walk's 33.70 cm, and the honest fix is a normal re-derived per solve
/// rather than baked, which is a change to the contract this crate has not
/// made yet.
///
/// `entrepierna` (`inseam`) and `largo_lateral` (`outseam`) are both
/// vertical drops rather than straight lines, and that much they do share,
/// unconditionally: the feet in Anny's A-pose stand about 20 cm either side
/// of the midline the waist and crotch centroids lie on, so a straight line
/// would fold that stance width into the measurement (2.6 cm on the inseam)
/// and make the number depend on the pose rather than on the body.
///
/// Where they stop is a different question, and PLAN-002's sealed decisions
/// give the two rows different answers. `entrepierna` is a height above the
/// floor — decision 3 puts `crotch_z = entrepierna` with the floor at zero
/// — measured down to the same `lo` the stature is measured down to, so the
/// parts still sum to the whole. `largo_lateral` runs waist to *ankle*:
/// decision 3-bis reads `ankle_z = waist_z - largo_lateral`, which under a
/// waist-to-floor reading would put `ankle_z` at zero and sink the loft's
/// foot caps below the ground.
///
/// That ankle is [`RingId::AnkleJoint`], a single-point landmark baked for
/// this measurement, and deliberately not [`RingId::Ankle`]'s girth ring:
/// `tobillo`'s ring is cut at the narrowest section the shin offers, which
/// on this mesh is a good half a shin above the ankle bone (14.4 cm above
/// the sole on the reference adult, against 7.0 cm for the landmark), so
/// ending the length there would lose that whole distance. This is the same
/// split `brazo` already makes at the wrist, for the same reason — see
/// `crate::anny_bake`'s doc, which also records why no lower narrowing
/// exists on this mesh for the girth ring to move down to.
///
/// `outseam` is still the waist's own drop rather than the path down the
/// outside of the leg ISO 8559 specifies, which is an honest approximation
/// the interface should state rather than claim ISO conformance for. The
/// hip-to-waist relationship is structural rather than incidental:
/// [`RingId::Hip`] is baked strictly above [`RingId::Crotch`] (the fork),
/// so `hip_drop` (`altura_cadera`) is always shorter than `rise` (`tiro`)
/// — see the crate's tests.
///
/// `upper_chest` (`pecho_alto`) cannot close either, and the reason is the
/// body rather than the ring. Its ring sits two centimetres below the
/// armpit apex (see `crate::anny_bake`'s doc); from there down to the waist
/// this mesh's horizontal section shrinks monotonically, so a `pecho_alto`
/// cut anywhere above `pecho`'s own ring necessarily reads *more* than
/// `pecho`. A tape asking for 94 against a `pecho` of 96 is asking for
/// less, which no honest height here can give: at `pecho`'s own ring — the
/// floor — the section already comes to 95.96 cm, and `pecho_alto` has no
/// lever of its own to close the rest.
///
/// `rise` (`tiro`) cannot close either, and there the obstacle is a lever
/// rather than a ring. It has none of its own, and the single lever with
/// real authority over it — `waisttohip-dist`, which stretches the whole
/// span below the waist — is claimed first by `altura_cadera`, which does
/// have one. Dropping [`RingId::Waist`] shortens both distances at once,
/// but only while that lever stays against its short stop; the moment it
/// comes off, it re-lengthens the span to hold `altura_cadera` on its dado
/// and gives `rise` back what the ring took. On the default tape the cut
/// that closes `altura_cadera` is already past that point (see
/// `crate::anny_bake`'s doc): the lever leaves its stop to close that row,
/// and `rise` comes out 0.67 cm wider than it does with no drop at all.
///
/// `back_length` (nape to waist) is the one measurement still reported
/// short of a generic anthropometric table after an honest search for a
/// better landmark: it lands between 33 and 43 cm on the reference bodies
/// against a generic ~47–52 cm table value. [`RingId::NapeBase`] is found
/// by direct surface search rather than assumed, and searching lower —
/// down toward the shoulder, the anatomically obvious direction for "the
/// base of the neck" — measurably made it worse, not better (see
/// `crate::anny_bake`'s doc for the evidence); reaching 47–52 cm this way
/// would require a "nape" already up in jaw or skull territory. What is
/// left after the waist drop above is this specific mesh's own proportion
/// — a short neck-to-waist span — rather than either landmark sitting in
/// the wrong place.
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
