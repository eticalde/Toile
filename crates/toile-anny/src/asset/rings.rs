/// One of the seventeen anatomical rings this asset carries.
///
/// Twelve are catalogue girths; five are landmark-only (`Crotch`, the two
/// `Shoulder*`, `ShoulderJoint` and `Elbow`) that no catalogue name reads
/// directly but the length formulas in `crate::measure` need as an anchor
/// point.
///
/// A ring is a fixed loop of points on the *neutral* body mesh (see
/// `RingPoint`): baked once, from the mesh alone, and never touched again —
/// [`crate::measure::measure`] only walks it, whatever the phenotype did to
/// the positions it is walked against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RingId {
    /// `cuello`: cut perpendicular to the neck's own axis, at the neck
    /// joint.
    Neck,
    /// `pecho_alto`: the highest horizontal cut that still separates the
    /// torso from both arms — see `crate::asset`'s doc on why this sits
    /// below the shoulder joint itself.
    UpperChest,
    /// `pecho`: the fullest horizontal section in a band around the chest.
    Bust,
    /// `bajo_pecho`: a fixed offset below [`RingId::Bust`].
    Underbust,
    /// `cintura`: the narrowest horizontal section in a band around the
    /// waist.
    Waist,
    /// `cadera`: the fullest horizontal section in a band around the hip.
    Hip,
    /// `muslo`: cut perpendicular to the thigh's own axis, near its top.
    Thigh,
    /// `rodilla`: cut perpendicular to the leg's local axis, at the knee.
    Knee,
    /// `tobillo`: cut perpendicular to the lower leg's axis, just above the
    /// ankle joint (see `crate::asset`'s doc on why not exactly at it).
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
    /// `entrepierna`. See `crate::asset`'s doc on why this is not simply
    /// the pelvis joint.
    Crotch,
    /// Landmark only: the right upper arm, cut at the same offset as
    /// [`RingId::UpperArm`] — see `crate::asset`'s doc on why that is also
    /// the closest valid cut to the shoulder joint. Anchors `hombros`, not
    /// `brazo` — see [`RingId::ShoulderJoint`] for that.
    ShoulderRight,
    /// Landmark only: the left counterpart of [`RingId::ShoulderRight`],
    /// anchoring `hombros`.
    ShoulderLeft,
    /// Landmark only: the right elbow, anchoring the bend in `brazo`.
    Elbow,
    /// Landmark only: a single point — the body vertex nearest the right
    /// shoulder joint on the neutral template — anchoring the start of
    /// `brazo`. Unlike [`RingId::ShoulderRight`], this is not a cut at all
    /// (the plane there cannot separate arm from torso), so it is the one
    /// ring with exactly one point rather than a closed loop; see
    /// `crate::asset`'s doc.
    ShoulderJoint,
}

impl RingId {
    /// How many rings the asset carries.
    pub const COUNT: usize = 17;

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
        RingId::ShoulderJoint,
    ];

    /// Whether this ring is a single point (see [`RingId::ShoulderJoint`])
    /// rather than a closed loop. A single-point "ring" is a landmark that
    /// cannot be cut as a real section; every other ring must close.
    pub fn is_single_point(self) -> bool {
        matches!(self, RingId::ShoulderJoint)
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

/// Where one ring's points sit in the asset's flat `ring_points` array.
///
/// Stored in [`RingId::ALL`] order, one entry per ring, so the id itself is
/// never written to the file — the reader's own fixed order carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RingRange {
    /// Index of this ring's first point in `Baked::ring_points`.
    pub offset: u32,
    /// How many consecutive points belong to this ring.
    pub length: u32,
}

impl RingRange {
    /// Its fixed byte footprint: an offset and a length.
    pub(super) const LEN: usize = 8;

    pub(super) fn write(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.offset.to_le_bytes());
        out.extend_from_slice(&self.length.to_le_bytes());
    }

    pub(super) fn read(bytes: &[u8]) -> Self {
        let u32_at =
            |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
        Self {
            offset: u32_at(0),
            length: u32_at(4),
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
    fn a_ring_range_round_trips_through_its_bytes() {
        let r = RingRange {
            offset: 7,
            length: 88,
        };
        let mut bytes = Vec::new();
        r.write(&mut bytes);
        assert_eq!(bytes.len(), RingRange::LEN);
        assert_eq!(RingRange::read(&bytes), r);
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
