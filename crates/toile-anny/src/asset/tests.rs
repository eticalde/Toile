use super::*;

fn sample() -> Baked {
    // One degenerate triangle and two rows are enough to exercise every
    // field's byte layout without pulling in the real 13,380-vertex body or
    // its 2.1M deltas.
    Baked {
        positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
        indices: vec![0, 1, 2],
        stations: vec![3, 7, 21],
        rows: vec![
            Row {
                kind: RowKind::Weighted { mask: 0b1010 },
                offset: 0,
                length: 2,
            },
            Row {
                kind: RowKind::Lever {
                    lever: 5,
                    incr: true,
                },
                offset: 2,
                length: 1,
            },
        ],
        deltas: vec![
            Delta {
                vertex: 0,
                dx: 10,
                dy: -10,
                dz: 0,
            },
            Delta {
                vertex: 1,
                dx: 0,
                dy: 5,
                dz: -5,
            },
            Delta {
                vertex: 2,
                dx: 1,
                dy: 1,
                dz: 1,
            },
        ],
        ring_ranges: vec![
            RingRange {
                offset: 0,
                length: 2,
            },
            RingRange {
                offset: 2,
                length: 1,
            },
        ],
        ring_points: vec![
            RingPoint {
                vertex_a: 0,
                vertex_b: 1,
                t: 0.5,
            },
            RingPoint {
                vertex_a: 1,
                vertex_b: 2,
                t: 0.25,
            },
            RingPoint {
                vertex_a: 2,
                vertex_b: 0,
                t: 0.75,
            },
        ],
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
    assert_eq!(back.rows, baked.rows);
    assert_eq!(back.deltas, baked.deltas);
    assert_eq!(back.ring_ranges, baked.ring_ranges);
    assert_eq!(back.ring_points, baked.ring_points);
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

#[test]
fn rejects_an_older_version_layout() {
    // A version-1 or version-2 file has a different header shape entirely
    // (no row/delta counts, or no ring counts), so this reader must refuse
    // it by version rather than misreading one section as another.
    let mut bytes = encode(&sample());
    bytes[8..12].copy_from_slice(&2u32.to_le_bytes());
    assert!(matches!(decode(&bytes), Err(DecodeError::BadVersion(2))));
}
