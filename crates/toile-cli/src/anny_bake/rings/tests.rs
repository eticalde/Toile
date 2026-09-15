use super::*;

fn synthetic_crossings(seed: usize) -> Vec<Crossing> {
    vec![(0, 1, 0.1 * seed as f64), (1, 2, 0.2), (2, 0, 0.3)]
}

#[test]
#[allow(
    clippy::float_cmp,
    reason = "halving an exact literal axis lands on an exact literal unit"
)]
fn flatten_lays_out_rings_contiguously_in_ring_id_order() {
    let rings: [Cut; RingId::COUNT] =
        std::array::from_fn(|i| Cut::along(synthetic_crossings(i), [0.0, 2.0, 0.0]));
    let (entries, points) = flatten_rings(rings);
    assert_eq!(entries.len(), RingId::COUNT);
    let mut expected_offset = 0u32;
    for e in &entries {
        assert_eq!(e.offset, expected_offset);
        assert_eq!(e.length, 3);
        // The plane travels with the ring, normalized: the asset stores the
        // direction the cut was taken along, never the caller's own scale.
        assert_eq!(e.normal, [0.0, 1.0, 0.0]);
        expected_offset += 3;
    }
    assert_eq!(points.len(), RingId::COUNT * 3);
}

#[test]
fn flatten_lays_out_paths_contiguously_in_path_id_order() {
    let paths: [Vec<Crossing>; PathId::COUNT] =
        std::array::from_fn(|i| synthetic_crossings(i)[..=i % 3].to_vec());
    let (entries, points) = flatten_paths(paths);
    let mut expected_offset = 0u32;
    for (i, e) in entries.iter().enumerate() {
        assert_eq!(e.offset, expected_offset);
        assert_eq!(e.length as usize, i % 3 + 1);
        expected_offset += e.length;
    }
    assert_eq!(points.len(), expected_offset as usize);
}

#[test]
#[should_panic(expected = "no joint group named")]
fn gather_panics_loudly_on_a_missing_joint() {
    gather(&[]);
}
