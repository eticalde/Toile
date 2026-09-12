/// One of the anatomical rings this asset carries.
///
/// Twelve are catalogue girths; the rest are landmark-only (`Crotch`, the
/// two `Shoulder*`, `Elbow`, `Acromion`, `WristJoint`, `AnkleJoint` and
/// `NapeBase`) that no catalogue name reads directly but the length formulas
/// in `crate::measure` need as an anchor point.
///
/// A ring is a fixed loop of points on the *neutral* body mesh (see
/// `RingPoint`): baked once, from the mesh alone, and never touched again —
/// [`crate::measure::measure`] only walks it, whatever the phenotype did to
/// the positions it is walked against.
///
/// A girth ring's own position and a length's end point need not be the same
/// vertex, and several pairs here deliberately are not. A girth has to be
/// cut where a plane can still separate the limb from the torso, or where
/// the section is one a tape could actually lie on; a length is free to end
/// on a single surface vertex at the joint itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RingId {
    /// `cuello`: cut perpendicular to the neck's own axis, at the neck
    /// joint.
    Neck,
    /// `pecho_alto`: just below the highest horizontal cut that still
    /// separates the torso from both arms, which is itself well below the
    /// shoulder joint — by the joint's own height the two surfaces have
    /// fused and no plane can part them.
    UpperChest,
    /// `pecho`: a horizontal cut at the height the CC0 breast target's own
    /// displacement-weighted apex falls at. Not a fullest-section scan: the
    /// neutral template carries no breast, so a scan could only ever find
    /// the ribcage.
    Bust,
    /// `bajo_pecho`: a fixed offset below [`RingId::Bust`].
    Underbust,
    /// `cintura`: a fixed drop below the narrowest horizontal section in a
    /// band around the waist — the waistline a garment sits on, which on
    /// this mesh is below the natural rib-cage indentation.
    Waist,
    /// `cadera`: the fullest horizontal section in a band around the hip.
    Hip,
    /// `muslo`: cut perpendicular to the thigh's own axis, near its top.
    Thigh,
    /// `rodilla`: cut perpendicular to the leg's local axis, at the knee.
    Knee,
    /// `tobillo`: cut perpendicular to the lower leg's axis, at the
    /// narrowest section the leg still has before the foot flares into it —
    /// which on this mesh is well up the shin rather than over the ankle
    /// bone. See [`RingId::AnkleJoint`] for the ankle a *length* runs to.
    Ankle,
    /// `brazo_contorno`: the fullest cut perpendicular to the upper arm's
    /// own axis.
    UpperArm,
    /// `muneca`: cut perpendicular to the forearm's axis, at the hand
    /// joint.
    Wrist,
    /// `cabeza`: the section with the largest left-right extent, above the
    /// ears.
    Head,
    /// Landmark only: a horizontal cut at the fork — the lowest section
    /// that still encloses both legs as one loop — anchoring `tiro` and
    /// `entrepierna`. Not the pelvis joint, which sits well above where the
    /// legs actually separate.
    Crotch,
    /// Landmark only: the right upper arm, cut at the same offset as
    /// [`RingId::UpperArm`], which is also the closest valid cut to the
    /// shoulder joint. Anchors `hombros`, not `brazo` — see
    /// [`RingId::Acromion`] for that.
    ShoulderRight,
    /// Landmark only: the left counterpart of [`RingId::ShoulderRight`],
    /// anchoring `hombros`.
    ShoulderLeft,
    /// Landmark only: the right elbow, anchoring the bend in `brazo`.
    Elbow,
    /// Landmark only: a single point — the highest body vertex within a
    /// small radius of the right shoulder joint, i.e. the top of the
    /// deltoid cap rather than the ball joint itself — anchoring the start
    /// of `brazo`. A length landmark can sit anywhere on the surface near
    /// the joint even where a girth ring cannot cut cleanly. Not a cut, so
    /// it is one of the rings with exactly one point rather than a closed
    /// loop.
    Acromion,
    /// Landmark only: a single point — the body vertex nearest the right
    /// hand joint — anchoring the end of `brazo`. Distinct from
    /// [`RingId::Wrist`], the girth ring three-quarters down the forearm:
    /// the forearm tapers smoothly into the hand, so a girth cut at the
    /// joint barely responds to the build and gender morphs.
    WristJoint,
    /// Landmark only: a single point — the body vertex nearest the right
    /// ankle joint, which on this mesh is the medial malleolus, the ankle
    /// bone a tailor's leg tape stops at — anchoring the bottom of
    /// `largo_lateral`. The leg's counterpart to [`RingId::WristJoint`],
    /// and distinct from [`RingId::Ankle`] for the same reason: that girth
    /// ring is cut at the narrowest section the shin offers, a good half a
    /// shin above the ankle bone, so a length ending there would fall short
    /// by that whole distance.
    AnkleJoint,
    /// Landmark only: a single point — the most posterior body vertex in a
    /// band centred on the neck joint's own height, near the sagittal
    /// midline — anchoring the top of `largo_espalda`: the nape, the C7
    /// vertebra at the back of the base of the neck. Distinct from
    /// [`RingId::Neck`]'s own most posterior point, because a ring cut at
    /// the joint is a full loop and its rearmost point sits partway around
    /// that loop rather than squarely on the spine.
    NapeBase,
}

impl RingId {
    /// How many rings the asset carries.
    pub const COUNT: usize = 20;

    /// Every ring, in the asset's own fixed storage order.
    pub const ALL: [RingId; RingId::COUNT] = [
        RingId::Neck,
        RingId::UpperChest,
        RingId::Bust,
        RingId::Underbust,
        RingId::Waist,
        RingId::Hip,
        RingId::Thigh,
        RingId::Knee,
        RingId::Ankle,
        RingId::UpperArm,
        RingId::Wrist,
        RingId::Head,
        RingId::Crotch,
        RingId::ShoulderRight,
        RingId::ShoulderLeft,
        RingId::Elbow,
        RingId::Acromion,
        RingId::WristJoint,
        RingId::AnkleJoint,
        RingId::NapeBase,
    ];

    /// Whether this ring is a single point rather than a closed loop. A
    /// single-point "ring" is a landmark that cannot be cut as a real
    /// section; every other ring must close.
    pub fn is_single_point(self) -> bool {
        matches!(
            self,
            RingId::Acromion | RingId::WristJoint | RingId::AnkleJoint | RingId::NapeBase
        )
    }
}

/// One point on a baked ring: a fixed fraction `t` along the fixed mesh edge
/// from body vertex `vertex_a` to `vertex_b`.
///
/// At runtime a ring point is `lerp(positions[vertex_a], positions[vertex_b],
/// t)` — never anything else — so a ring follows the mesh exactly as it
/// morphs, with no search and no per-frame intersection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RingPoint {
    /// The edge's first body vertex.
    pub vertex_a: u16,
    /// The edge's second body vertex.
    pub vertex_b: u16,
    /// The fraction from `vertex_a` toward `vertex_b`, in `[0, 1]`.
    pub t: f32,
}

impl RingPoint {
    /// Its fixed byte footprint: two `u16` indices and an `f32` fraction.
    pub(super) const LEN: usize = 8;

    pub(super) fn write(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.vertex_a.to_le_bytes());
        out.extend_from_slice(&self.vertex_b.to_le_bytes());
        out.extend_from_slice(&self.t.to_le_bytes());
    }

    pub(super) fn read(bytes: &[u8]) -> Self {
        let u16_at = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
        let f32_at =
            |i: usize| f32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
        Self {
            vertex_a: u16_at(0),
            vertex_b: u16_at(2),
            t: f32_at(4),
        }
    }
}

/// One ring's row in the asset's ring table: where its points sit in the
/// flat `ring_points` array, and the plane the bake cut them on.
///
/// Stored in [`RingId::ALL`] order, one entry per ring, so the id itself is
/// never written to the file — the reader's own fixed order carries it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RingEntry {
    /// Index of this ring's first point in `Baked::ring_points`.
    pub offset: u32,
    /// How many consecutive points belong to this ring.
    pub length: u32,
    /// The unit normal of the plane this ring was cut on.
    ///
    /// A ring's points are exactly coplanar on the template — they are
    /// where that plane crossed the mesh — but a morph drags them out of
    /// it, and a chord walk that follows them out of plane measures the
    /// wander as if it were girth. Keeping the plane here lets
    /// [`crate::measure`] sum the girth *in* it whatever the phenotype did.
    ///
    /// It is the plane the ring was *cut* on, which the trunk keeps under a
    /// morph and a limb does not: a limb girth is read across a slightly
    /// oblique section, a little under a tape lying flat on the limb.
    ///
    /// Zero for the single-point landmarks ([`RingId::is_single_point`]),
    /// which are surface vertices found by search rather than cuts and so
    /// have no plane at all. Projecting against a zero normal changes
    /// nothing, which is the right answer for a ring with no chord to walk.
    pub normal: [f32; 3],
}

impl RingEntry {
    /// Its fixed byte footprint: an offset, a length and a plane normal.
    pub(super) const LEN: usize = 20;

    pub(super) fn write(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.offset.to_le_bytes());
        out.extend_from_slice(&self.length.to_le_bytes());
        for c in self.normal {
            out.extend_from_slice(&c.to_le_bytes());
        }
    }

    pub(super) fn read(bytes: &[u8]) -> Self {
        let u32_at =
            |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
        let f32_at =
            |i: usize| f32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
        Self {
            offset: u32_at(0),
            length: u32_at(4),
            normal: [f32_at(8), f32_at(12), f32_at(16)],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ring_point_round_trips_through_its_bytes() {
        let p = RingPoint {
            vertex_a: 12_345,
            vertex_b: 42,
            t: 0.375,
        };
        let mut bytes = Vec::new();
        p.write(&mut bytes);
        assert_eq!(bytes.len(), RingPoint::LEN);
        assert_eq!(RingPoint::read(&bytes), p);
    }

    #[test]
    fn a_ring_entry_round_trips_through_its_bytes() {
        let r = RingEntry {
            offset: 7,
            length: 88,
            normal: [0.0, -1.0, 0.5],
        };
        let mut bytes = Vec::new();
        r.write(&mut bytes);
        assert_eq!(bytes.len(), RingEntry::LEN);
        assert_eq!(RingEntry::read(&bytes), r);
    }

    #[test]
    fn ring_id_all_has_no_duplicate_and_matches_count() {
        assert_eq!(RingId::ALL.len(), RingId::COUNT);
        for (i, a) in RingId::ALL.iter().enumerate() {
            for b in &RingId::ALL[i + 1..] {
                assert_ne!(*a as u8, *b as u8, "duplicate ring id");
            }
        }
    }
}
