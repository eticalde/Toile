/// One vertex's contribution within a row's delta run: which body vertex it
/// moves, and by how much.
///
/// The three axes are quantized as thousandths of a decimetre — the
/// source `.target.gz` text carries at most three decimal places in
/// decimetres, so `round(value * 1000)` is exact, and the baker asserts
/// that losslessness file by file rather than trusting it. A delta in
/// metres is `component as f64 / 10_000.0` (÷1000 back to decimetres,
/// ×0.1 to metres).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Delta {
    /// The body vertex this delta moves (see `crate` for the 13,380 count).
    pub vertex: u16,
    /// Quantized x displacement; see the struct doc for the unit.
    pub dx: i16,
    /// Quantized y displacement; see the struct doc for the unit.
    pub dy: i16,
    /// Quantized z displacement; see the struct doc for the unit.
    pub dz: i16,
}

impl Delta {
    /// Its fixed byte footprint in the asset: one `u16` and three `i16`.
    pub(super) const LEN: usize = 8;

    pub(super) fn write(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.vertex.to_le_bytes());
        out.extend_from_slice(&self.dx.to_le_bytes());
        out.extend_from_slice(&self.dy.to_le_bytes());
        out.extend_from_slice(&self.dz.to_le_bytes());
    }

    pub(super) fn read(bytes: &[u8]) -> Self {
        let u16_at = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
        let i16_at = |i: usize| i16::from_le_bytes([bytes[i], bytes[i + 1]]);
        Self {
            vertex: u16_at(0),
            dx: i16_at(2),
            dy: i16_at(4),
            dz: i16_at(6),
        }
    }
}

/// What a baked row's weight is computed from.
///
/// Either the 26-bit phenotype mask of a weighted macrodetail/breast row,
/// or the identity of a `measure-*` lever row (baked in this slice, not yet
/// applied to the mesh — see `crate::phenotype::Phenotype`'s doc).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// A macrodetail or breast target: `mask` has bit `i` set for every
    /// `crate::phenotype::KEYS[i]` its filename mentioned.
    Weighted {
        /// The 26-bit phenotype mask (bits above 25 are always zero).
        mask: u32,
    },
    /// One half of a `measure-*` lever pair.
    Lever {
        /// Index into `crate::phenotype::LEVERS`.
        lever: u8,
        /// `true` for the `-incr` file, `false` for `-decr`.
        incr: bool,
    },
}

/// One target file's place in the asset: what drives its weight, and where
/// its delta run sits in the flat `Baked::deltas` array.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    /// See [`RowKind`].
    pub kind: RowKind,
    /// Index of this row's first delta in `Baked::deltas`.
    pub offset: u32,
    /// How many consecutive deltas belong to this row.
    pub length: u32,
}

impl Row {
    /// Its fixed byte footprint: a kind tag, a 4-byte mask, a lever id and
    /// direction byte, then the offset and length — 15 bytes regardless of
    /// which `RowKind` it carries, so the row table can be indexed without
    /// scanning it.
    pub(super) const LEN: usize = 15;

    pub(super) fn write(self, out: &mut Vec<u8>) {
        match self.kind {
            RowKind::Weighted { mask } => {
                out.push(0);
                out.extend_from_slice(&mask.to_le_bytes());
                out.push(0);
                out.push(0);
            }
            RowKind::Lever { lever, incr } => {
                out.push(1);
                out.extend_from_slice(&0u32.to_le_bytes());
                out.push(lever);
                out.push(u8::from(incr));
            }
        }
        out.extend_from_slice(&self.offset.to_le_bytes());
        out.extend_from_slice(&self.length.to_le_bytes());
    }

    /// # Panics
    /// If the kind tag byte is neither 0 nor 1: the shipped asset is baked
    /// by [`Row::write`] and tested against this reader, so that would mean
    /// a corrupted or hand-edited file.
    pub(super) fn read(bytes: &[u8]) -> Self {
        let u32_at =
            |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
        let kind = match bytes[0] {
            0 => RowKind::Weighted { mask: u32_at(1) },
            1 => RowKind::Lever {
                lever: bytes[5],
                incr: bytes[6] != 0,
            },
            other => panic!("unknown row kind byte {other}"),
        };
        Self {
            kind,
            offset: u32_at(7),
            length: u32_at(11),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_delta_round_trips_through_its_bytes() {
        let d = Delta {
            vertex: 12_345,
            dx: -32_000,
            dy: 0,
            dz: 7,
        };
        let mut bytes = Vec::new();
        d.write(&mut bytes);
        assert_eq!(bytes.len(), Delta::LEN);
        assert_eq!(Delta::read(&bytes), d);
    }

    #[test]
    fn both_row_kinds_round_trip_through_their_bytes() {
        for row in [
            Row {
                kind: RowKind::Weighted { mask: 0x0300_0021 },
                offset: 7,
                length: 42,
            },
            Row {
                kind: RowKind::Lever {
                    lever: 19,
                    incr: false,
                },
                offset: 100,
                length: 3,
            },
        ] {
            let mut bytes = Vec::new();
            row.write(&mut bytes);
            assert_eq!(bytes.len(), Row::LEN);
            assert_eq!(Row::read(&bytes), row);
        }
    }
}
