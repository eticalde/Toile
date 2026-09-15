/// One catalogue length the asset carries as a line on the skin.
///
/// A path is cut once from the neutral template, the way a ring is: the
/// section a plane leaves on the mesh, kept from one landmark to another as an
/// open run of [`super::RingPoint`]s. [`crate::measure::measure`] sums its
/// chords over the morphed positions, so the length follows the body the way a
/// tape pressed to it does, and every point stays on the mesh edge it was cut
/// on. Consecutive points always lie on the edges of one triangle, so the
/// chord between them lies in that triangle on every body.
///
/// The leg and arm paths all run down the body's right side, toward `-x`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PathId {
    /// `entrepierna`: down the inside of the right leg, from the fork, over
    /// the inner ankle bone and down the side of the heel to where it turns
    /// under toward the sole. See [`PathId::ends_on_floor`] for the rest of
    /// the way down.
    Inseam,
    /// `largo_lateral`: down the outside of the right leg, from the waist at
    /// the side, over the hip, to the outer ankle at the inner ankle bone's
    /// height. Cut on the same plane as [`PathId::Inseam`].
    Outseam,
    /// `tiro`: the start of [`PathId::Outseam`], point for point, from the
    /// waist down the side to the fork's height.
    Rise,
    /// `altura_cadera`: the start of [`PathId::Outseam`], point for point,
    /// from the waist down the side to the hip ring's height.
    HipDrop,
    /// `largo_espalda`: down the spine on the body's mirror plane, from the
    /// nape's height to the waist's.
    Back,
    /// `brazo`: from the right shoulder point across the outside of the upper
    /// arm to the elbow's point, then down the back of the forearm to the
    /// wrist.
    Arm,
    /// `hombros`: across the back, from one shoulder point to the other.
    Shoulders,
}

impl PathId {
    /// How many paths the asset carries.
    pub const COUNT: usize = 7;

    /// Every path, in the asset's own fixed storage order.
    pub const ALL: [PathId; PathId::COUNT] = [
        PathId::Inseam,
        PathId::Outseam,
        PathId::Rise,
        PathId::HipDrop,
        PathId::Back,
        PathId::Arm,
        PathId::Shoulders,
    ];

    /// Whether the length goes on past the path's last point, straight down
    /// to the floor the body stands on.
    ///
    /// Only the inseam does. The trade takes it to the floor, and the skin
    /// stops short of the floor where the heel turns under toward the sole:
    /// from there the tape falls plumb, clear of a foot that curves away
    /// beneath it.
    pub fn ends_on_floor(self) -> bool {
        matches!(self, PathId::Inseam)
    }
}

/// One path's row in the asset's path table: where its points sit in the flat
/// `path_points` array.
///
/// Stored in [`PathId::ALL`] order, one entry per path, so the id itself is
/// never written to the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathEntry {
    /// Index of this path's first point in `Baked::path_points`.
    pub offset: u32,
    /// How many consecutive points belong to this path.
    pub length: u32,
}

impl PathEntry {
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
    fn a_path_entry_round_trips_through_its_bytes() {
        let e = PathEntry {
            offset: 70_000,
            length: 85,
        };
        let mut bytes = Vec::new();
        e.write(&mut bytes);
        assert_eq!(bytes.len(), PathEntry::LEN);
        assert_eq!(PathEntry::read(&bytes), e);
    }

    #[test]
    fn path_id_all_has_no_duplicate_and_matches_count() {
        for (i, a) in PathId::ALL.iter().enumerate() {
            assert_eq!(*a as usize, i, "ALL is in storage order");
        }
    }
}
