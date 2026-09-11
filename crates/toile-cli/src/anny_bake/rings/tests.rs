use super::*;

fn synthetic_loop(seed: usize) -> Vec<Crossing> {
    vec![(0, 1, 0.1 * seed as f64), (1, 2, 0.2), (2, 0, 0.3)]
}

#[test]
fn flatten_lays_out_ranges_contiguously_in_ring_id_order() {
    let rings: [Vec<Crossing>; RingId::COUNT] = std::array::from_fn(synthetic_loop);
    let (ranges, points) = flatten(rings);
    assert_eq!(ranges.len(), RingId::COUNT);
    let mut expected_offset = 0u32;
    for r in &ranges {
        assert_eq!(r.offset, expected_offset);
        assert_eq!(r.length, 3);
        expected_offset += 3;
    }
    assert_eq!(points.len(), RingId::COUNT * 3);
}

#[test]
#[should_panic(expected = "no joint group named")]
fn gather_panics_loudly_on_a_missing_joint() {
    gather(&[]);
}
