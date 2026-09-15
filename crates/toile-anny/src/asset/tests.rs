use super::*;

fn sample() -> Baked {
    // One degenerate triangle and two rows are enough to exercise every
    // field's byte layout without pulling in the real 13,380-vertex body or
    // its 2.1M deltas.
    let point = |vertex_a, vertex_b, t| RingPoint {
        vertex_a,
        vertex_b,
        t,
    };
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
        ring_entries: vec![RingEntry {
            offset: 0,
            length: 3,
            normal: [0.0, 1.0, 0.0],
        }],
        ring_points: vec![point(0, 1, 0.5), point(1, 2, 0.25), point(2, 0, 0.75)],
        path_entries: vec![
            PathEntry {
                offset: 0,
                length: 2,
            },
            PathEntry {
                offset: 2,
                length: 2,
            },
        ],
        path_points: vec![
            point(0, 1, 0.125),
            point(1, 2, 0.5),
            point(2, 0, 0.0),
            point(0, 2, 1.0),
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
    assert_eq!(back.ring_entries, baked.ring_entries);
    assert_eq!(back.ring_points, baked.ring_points);
    assert_eq!(back.path_entries, baked.path_entries);
    assert_eq!(back.path_points, baked.path_points);
}

#[test]
fn rejects_a_flipped_payload_byte() {
    let mut bytes = encode(&sample());
    let last = bytes.len() - 1;
    bytes[last] ^= 0xFF;
    assert!(matches!(decode(&bytes), Err(DecodeError::BadHash)));
}

#[test]
fn rejects_a_payload_one_byte_short() {
    let mut bytes = encode(&sample());
    bytes.pop();
    assert!(matches!(decode(&bytes), Err(DecodeError::TruncatedPayload)));
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

/// An older layout can be self-consistent enough to pass the length check —
/// a header one field shorter reads its hash as a count, a table with fewer
/// entries than an id indexes decodes perfectly well — so the reader refuses
/// by version rather than risk misreading one section as another, or
/// panicking later on a table it accepted.
#[test]
fn rejects_the_layout_before_paths() {
    let mut bytes = encode(&sample());
    bytes[8..12].copy_from_slice(&5u32.to_le_bytes());
    assert!(matches!(decode(&bytes), Err(DecodeError::BadVersion(5))));
}
