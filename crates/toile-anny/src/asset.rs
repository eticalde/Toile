mod paths;
mod rings;
mod rows;

pub use paths::{PathEntry, PathId};
pub use rings::{RingEntry, RingId, RingPoint};
pub(crate) use rows::DELTA_PER_METRE;
pub use rows::{Delta, Row, RowKind};

/// The bytes every asset opens with, so a truncated or unrelated file is
/// refused before any length arithmetic runs against it.
const MAGIC: [u8; 8] = *b"TOANNY01";

/// Bumped whenever the payload layout changes. Only this exact version
/// decodes: an asset written by any other is refused outright rather than
/// read as a shape this reader was not built for. Nothing here reads an
/// older layout, so the bump is the whole compatibility story and the
/// history of what each one added belongs in `git log`.
///
/// The ring and path counts are part of that shape even though the header
/// carries them: a table one entry short is self-consistent enough to pass
/// every length and hash check here and then panic in [`crate::measure`],
/// which indexes both tables by id and expects every entry.
const VERSION: u32 = 6;

/// The header's fixed size in bytes: the magic, nine `u32` fields, the hash.
const HEADER_LEN: usize = 8 + 9 * 4 + 8;

/// FNV-1a's offset basis and prime. Mirrored from `toile_engine::golden`
/// rather than shared: this crate cannot depend on the engine, which depends
/// on it, and the mix itself is two literals, not a chain worth a crate.
const FNV_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0100_0000_01b3;

/// The body as the baker hands it over, and as the reader hands it back.
///
/// Metres, y-up, centred, CCW-outward triangles, one station tag per vertex
/// — the template — plus every baked target's row and delta run, in the
/// fixed order the baker discovered them in, plus the girth rings and the
/// length paths cut once from the neutral template (see [`RingId`] and
/// [`PathId`]). Normals are not part of the asset: they are cheap to
/// recompute, and storing them would let the two drift apart.
///
/// The payload is stored raw and uncompressed, `include_bytes!`-embedded
/// and committed rather than fetched or decompressed at load time: a
/// deliberate trade for `toile-anny` keeping zero runtime dependencies. The
/// size is what was traded away: about 17 MB shipped, of which the deltas
/// are nearly all.
pub struct Baked {
    /// Vertex positions as xyz triples.
    pub positions: Vec<f32>,
    /// CCW triangle indices into `positions`.
    pub indices: Vec<u32>,
    /// The `Station` tag of every vertex.
    pub stations: Vec<u8>,
    /// One entry per baked target file, in bake order. A row's `offset`
    /// and `length` (see [`Row`]) point into `deltas`.
    pub rows: Vec<Row>,
    /// Every row's delta entries, concatenated in row order.
    pub deltas: Vec<Delta>,
    /// One entry per [`RingId`], in [`RingId::ALL`] order. An entry's
    /// `offset` and `length` point into `ring_points`.
    pub ring_entries: Vec<RingEntry>,
    /// Every ring's points, concatenated in `ring_entries` order.
    pub ring_points: Vec<RingPoint>,
    /// One entry per [`PathId`], in [`PathId::ALL`] order. An entry's
    /// `offset` and `length` point into `path_points`.
    pub path_entries: Vec<PathEntry>,
    /// Every path's points, concatenated in `path_entries` order.
    pub path_points: Vec<RingPoint>,
}

impl Baked {
    /// The delta entries one row owns.
    ///
    /// Every reader of `deltas` wants exactly this slice, and each one that
    /// cut it for itself was a place the `offset`-plus-`length` convention
    /// could be read one way while the writer meant another.
    pub(crate) fn row_deltas(&self, row: &Row) -> &[Delta] {
        let start = row.offset as usize;
        &self.deltas[start..start + row.length as usize]
    }
}

/// Why a byte slice would not decode as an asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// Fewer bytes than a header needs.
    TooShort,
    /// The first eight bytes are not [`MAGIC`].
    BadMagic,
    /// The format version is not one this reader knows.
    BadVersion(u32),
    /// The header's counts do not match the bytes that follow.
    TruncatedPayload,
    /// The payload's hash does not match the one the header carries.
    BadHash,
}

/// Byte-wise FNV-1a: the offset basis mixed with one byte at a time.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h = FNV_BASIS;
    for &b in bytes {
        h = (h ^ u64::from(b)).wrapping_mul(FNV_PRIME);
    }
    h
}

/// Packs a baked body into the asset's byte layout, hashing the payload as it
/// writes the header.
///
/// # Layout
/// ```text
/// offset  bytes     field
/// 0       8         magic: b"TOANNY01"
/// 8       4         format version, u32 LE (6)
/// 12      4         vertex count V, u32 LE
/// 16      4         triangle count T, u32 LE
/// 20      4         row count R, u32 LE
/// 24      4         delta count D, u32 LE
/// 28      4         ring entry count G, u32 LE (always RingId::COUNT)
/// 32      4         ring point count P, u32 LE
/// 36      4         path entry count H, u32 LE (always PathId::COUNT)
/// 40      4         path point count Q, u32 LE
/// 44      8         FNV-1a hash of everything from offset 52 on, u64 LE
/// 52      12*V      positions: V vertex xyz triples, f32 LE, metres, y-up
/// ...     12*T      indices: T triangle index triples, u32 LE
/// ...     V         stations: one Station tag per vertex, u8
/// ...     15*R      row table: R rows, see `Row`/`RowKind`'s byte layout
/// ...     8*D       delta runs: D deltas, see `Delta`'s byte layout
/// ...     20*G      ring table: G entries, see `RingEntry`'s byte layout
/// ...     8*P       ring points: P points, see `RingPoint`'s byte layout
/// ...     8*H       path table: H entries, see `PathEntry`'s byte layout
/// ...     8*Q       path points: Q points, laid out as ring points are
/// ```
pub fn encode(baked: &Baked) -> Vec<u8> {
    debug_assert_eq!(baked.stations.len(), baked.positions.len() / 3);
    let counts = [
        baked.positions.len() / 3,
        baked.indices.len() / 3,
        baked.rows.len(),
        baked.deltas.len(),
        baked.ring_entries.len(),
        baked.ring_points.len(),
        baked.path_entries.len(),
        baked.path_points.len(),
    ];

    let mut payload = Vec::new();
    for f in &baked.positions {
        payload.extend_from_slice(&f.to_le_bytes());
    }
    for i in &baked.indices {
        payload.extend_from_slice(&i.to_le_bytes());
    }
    payload.extend_from_slice(&baked.stations);
    baked.rows.iter().for_each(|r| r.write(&mut payload));
    baked.deltas.iter().for_each(|d| d.write(&mut payload));
    baked
        .ring_entries
        .iter()
        .for_each(|e| e.write(&mut payload));
    baked.ring_points.iter().for_each(|p| p.write(&mut payload));
    baked
        .path_entries
        .iter()
        .for_each(|e| e.write(&mut payload));
    baked.path_points.iter().for_each(|p| p.write(&mut payload));

    let mut out = Vec::with_capacity(HEADER_LEN + payload.len());
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    for count in counts {
        out.extend_from_slice(&(count as u32).to_le_bytes());
    }
    out.extend_from_slice(&fnv1a(&payload).to_le_bytes());
    out.extend_from_slice(&payload);
    out
}

/// The format version an asset declares, and the fingerprint of its payload.
///
/// Read straight off the header rather than by decoding: a caller filing
/// something under "the body data this build generates meshes from" wants the
/// two numbers, not the seventeen megabytes behind them. `None` for bytes too
/// short to carry a header or not opening with [`MAGIC`].
pub fn stamp(bytes: &[u8]) -> Option<(u32, u64)> {
    if bytes.len() < HEADER_LEN || bytes[0..8] != MAGIC {
        return None;
    }
    let hash = u64::from_le_bytes(std::array::from_fn(|i| bytes[44 + i]));
    Some((le_u32(bytes, 8), hash))
}

/// Reads a little-endian `u32` from the four bytes at `at`, which the caller
/// has already checked lie inside `bytes`.
fn le_u32(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// Splits `N`-byte records off the front of `bytes`, `count` of them, and
/// reads each with `read`: one table of the payload.
fn records<const N: usize, T>(
    bytes: &mut &[u8],
    count: usize,
    read: impl Fn(&[u8; N]) -> T,
) -> Vec<T> {
    let (table, rest) = bytes.split_at(count * N);
    *bytes = rest;
    table.as_chunks::<N>().0.iter().map(read).collect()
}

/// Unpacks an asset's bytes back into a [`Baked`] body.
///
/// # Errors
/// One of [`DecodeError`]'s reasons: a file this small only comes from a
/// build mistake, since the shipped bytes are baked and tested against this
/// same reader.
///
/// # Panics
/// Never, in practice: the length check ahead of every split is what the
/// slicing below would otherwise need to guard against, so it can never see a
/// slice the wrong size.
pub fn decode(bytes: &[u8]) -> Result<Baked, DecodeError> {
    if bytes.len() < HEADER_LEN {
        return Err(DecodeError::TooShort);
    }
    if bytes[0..8] != MAGIC {
        return Err(DecodeError::BadMagic);
    }
    let version = le_u32(bytes, 8);
    if version != VERSION {
        return Err(DecodeError::BadVersion(version));
    }
    let [v, t, r, d, g, p, h, q] = std::array::from_fn(|i| le_u32(bytes, 12 + 4 * i) as usize);
    let hash = u64::from_le_bytes(std::array::from_fn(|i| bytes[44 + i]));

    let mut payload = &bytes[HEADER_LEN..];
    let want_len = v * 12
        + t * 12
        + v
        + r * Row::LEN
        + d * Delta::LEN
        + g * RingEntry::LEN
        + p * RingPoint::LEN
        + h * PathEntry::LEN
        + q * RingPoint::LEN;
    if payload.len() != want_len {
        return Err(DecodeError::TruncatedPayload);
    }
    if fnv1a(payload) != hash {
        return Err(DecodeError::BadHash);
    }

    Ok(Baked {
        positions: records::<4, _>(&mut payload, v * 3, |c| f32::from_le_bytes(*c)),
        indices: records::<4, _>(&mut payload, t * 3, |c| u32::from_le_bytes(*c)),
        stations: records::<1, _>(&mut payload, v, |c| c[0]),
        rows: records::<{ Row::LEN }, _>(&mut payload, r, |c| Row::read(c)),
        deltas: records::<{ Delta::LEN }, _>(&mut payload, d, |c| Delta::read(c)),
        ring_entries: records::<{ RingEntry::LEN }, _>(&mut payload, g, |c| RingEntry::read(c)),
        ring_points: records::<{ RingPoint::LEN }, _>(&mut payload, p, |c| RingPoint::read(c)),
        path_entries: records::<{ PathEntry::LEN }, _>(&mut payload, h, |c| PathEntry::read(c)),
        path_points: records::<{ RingPoint::LEN }, _>(&mut payload, q, |c| RingPoint::read(c)),
    })
}

#[cfg(test)]
mod tests;
