/// One of the twelve catalogue girths, each carried as a ring.
///
/// A ring is a fixed loop of points on the *neutral* body mesh (see
/// `RingPoint`): baked once, from the mesh alone, and never touched again —
/// [`crate::measure::measure`] only walks it, whatever the phenotype did to
/// the positions it is walked against. The catalogue's lengths are not rings
/// but open lines on the skin: see [`super::PathId`].
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
    /// bone.
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
}

impl RingId {
    /// How many rings the asset carries.
    pub const COUNT: usize = 12;

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
    ];
}

/// One point on a baked ring or path: a fixed fraction `t` along the fixed
/// mesh edge from body vertex `vertex_a` to `vertex_b`.
///
/// At runtime a point is `lerp(positions[vertex_a], positions[vertex_b], t)` —
/// never anything else — so it follows the mesh exactly as it morphs, with no
/// search and no per-frame intersection.
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
    fn ring_id_all_is_in_storage_order() {
        for (i, a) in RingId::ALL.iter().enumerate() {
            assert_eq!(*a as usize, i, "ALL is in storage order");
        }
    }
}
