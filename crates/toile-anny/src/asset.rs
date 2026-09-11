/// The bytes every asset opens with, so a truncated or unrelated file is
/// refused before any length arithmetic runs against it.
const MAGIC: [u8; 8] = *b"TOANNY01";

/// Bumped whenever the payload layout changes, so a reader never misreads a
/// shape it was not built for.
const VERSION: u32 = 1;

/// The header's fixed size in bytes, ahead of the three payload arrays.
const HEADER_LEN: usize = 8 + 4 + 4 + 4 + 8;

/// FNV-1a's offset basis and prime. Mirrored from `toile_engine::golden`
/// rather than shared: this crate cannot depend on the engine, which depends
/// on it, and the mix itself is two literals, not a chain worth a crate.
const FNV_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0100_0000_01b3;

/// The body as the baker hands it over, and as the reader hands it back.
///
/// Metres, y-up, centred, CCW-outward triangles, one station tag per vertex.
/// Normals are not part of the asset — they are cheap to recompute and
/// storing them would let the two drift apart.
pub struct Baked {
    /// Vertex positions as xyz triples.
    pub positions: Vec<f32>,
    /// CCW triangle indices into `positions`.
    pub indices: Vec<u32>,
    /// The `Station` tag of every vertex.
    pub stations: Vec<u8>,
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
pub fn encode(baked: &Baked) -> Vec<u8> {
    let vertex_count = (baked.positions.len() / 3) as u32;
    let triangle_count = (baked.indices.len() / 3) as u32;
    debug_assert_eq!(baked.stations.len(), vertex_count as usize);

    let mut payload = Vec::with_capacity(
        baked.positions.len() * 4 + baked.indices.len() * 4 + baked.stations.len(),
    );
    for f in &baked.positions {
        payload.extend_from_slice(&f.to_le_bytes());
    }
    for i in &baked.indices {
        payload.extend_from_slice(&i.to_le_bytes());
    }
    payload.extend_from_slice(&baked.stations);

    let mut out = Vec::with_capacity(HEADER_LEN + payload.len());
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&vertex_count.to_le_bytes());
    out.extend_from_slice(&triangle_count.to_le_bytes());
    out.extend_from_slice(&fnv1a(&payload).to_le_bytes());
    out.extend_from_slice(&payload);
    out
}

/// Reads a little-endian `u32` from the first four bytes of `field`, a name
/// used only in the panic message a length mismatch would raise.
fn le_u32(field: &str, bytes: &[u8]) -> u32 {
    let chunk = bytes.first_chunk::<4>().unwrap_or_else(|| {
        panic!(
            "{field} needs 4 bytes, got {}: checked against HEADER_LEN above",
            bytes.len()
        )
    });
    u32::from_le_bytes(*chunk)
}

/// Reads a little-endian `u64`, the same way [`le_u32`] reads a `u32`.
fn le_u64(field: &str, bytes: &[u8]) -> u64 {
    let chunk = bytes.first_chunk::<8>().unwrap_or_else(|| {
        panic!(
            "{field} needs 8 bytes, got {}: checked against HEADER_LEN above",
            bytes.len()
        )
    });
    u64::from_le_bytes(*chunk)
}

/// Unpacks an asset's bytes back into positions, indices and station tags.
///
/// # Errors
/// One of [`DecodeError`]'s reasons: a file this small only comes from a
/// build mistake, since the shipped bytes are baked and tested against this
/// same reader.
///
/// # Panics
/// Never, in practice: the internal length checks above every fixed-size
/// read are what the panicking accessors below would otherwise need to
/// guard against, so they can never see a slice the wrong size.
pub fn decode(bytes: &[u8]) -> Result<Baked, DecodeError> {
    if bytes.len() < HEADER_LEN {
        return Err(DecodeError::TooShort);
    }
    if bytes[0..8] != MAGIC {
        return Err(DecodeError::BadMagic);
    }
    let version = le_u32("version", &bytes[8..12]);
    if version != VERSION {
        return Err(DecodeError::BadVersion(version));
    }
    let vertex_count = le_u32("vertex_count", &bytes[12..16]) as usize;
    let triangle_count = le_u32("triangle_count", &bytes[16..20]) as usize;
    let hash = le_u64("hash", &bytes[20..28]);

    let payload = &bytes[HEADER_LEN..];
    let want_len = vertex_count * 3 * 4 + triangle_count * 3 * 4 + vertex_count;
    if payload.len() != want_len {
        return Err(DecodeError::TruncatedPayload);
    }
    if fnv1a(payload) != hash {
        return Err(DecodeError::BadHash);
    }

    let (pos_bytes, rest) = payload.split_at(vertex_count * 3 * 4);
    let (idx_bytes, stations) = rest.split_at(triangle_count * 3 * 4);
    let positions = pos_bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes(*c))
        .collect();
    let indices = idx_bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| u32::from_le_bytes(*c))
        .collect();

    Ok(Baked {
        positions,
        indices,
        stations: stations.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Baked {
        // One degenerate triangle is enough to exercise every field's byte
        // layout without pulling in the real 13,380-vertex body.
        Baked {
            positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            indices: vec![0, 1, 2],
            stations: vec![3, 7, 21],
        }
    }

    #[test]
    fn round_trips_a_small_baked_body() {
        let baked = sample();
        let bytes = encode(&baked);
        let back = decode(&bytes).expect("a freshly encoded asset decodes");
        assert_eq!(back.positions, baked.positions);
        assert_eq!(back.indices, baked.indices);
        assert_eq!(back.stations, baked.stations);
    }

    #[test]
    fn rejects_a_flipped_payload_byte() {
        let mut bytes = encode(&sample());
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF;
        assert!(matches!(decode(&bytes), Err(DecodeError::BadHash)));
    }

    #[test]
    fn rejects_a_foreign_file() {
        let long_enough_but_foreign = [0u8; HEADER_LEN];
        assert!(matches!(
            decode(&long_enough_but_foreign),
            Err(DecodeError::BadMagic)
        ));
        assert!(matches!(decode(&[]), Err(DecodeError::TooShort)));
    }
}
