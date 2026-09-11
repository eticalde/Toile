#![allow(missing_docs, reason = "a test crate publishes no API surface")]

/// Measuring the reference adult Anny body (the same phenotype
/// `anny_golden.rs` pins the mesh of) must always produce these 20
/// centimetre values, hashed together.
///
/// This is the third vertical slice's golden: the first pinned the bare
/// neutral template (`0xb3f8_8dd8_6abc_c96f`), the second pinned the
/// phenotype-driven mesh (`0x6254_3e96_3147_07eb`); this one pins where the
/// seventeen measurement rings land on that same mesh. A change to a
/// ring's placement — a different band, a different step, a different
/// loop picked — moves this hash without moving either mesh golden, since
/// measuring only reads positions, never writes them. It moved once
/// already, from `0x065f_1605_154b_0935`, when the crotch landmark was
/// corrected from the pelvis joint to the true leg fork, the hip band was
/// constrained to sit above that fork, the neck-base landmark moved from
/// the ring's centroid to its most posterior point, `brazo` was anchored
/// on the true shoulder joint instead of the girth ring, and `muneca`
/// backed off from the hand joint to a point that actually responds to
/// the build and gender morphs.
///
/// Take the new value from this assertion and update the constant in the
/// same commit, saying why.
#[test]
fn the_anny_measures_hash_to_a_fixed_value() {
    assert_eq!(
        toile_engine::golden::anny_measures_hash(),
        0x3728_9125_b54b_0935,
        "the Anny body's measured values changed bits: a ring moved on \
         purpose, or a dependency drifted under it"
    );
}
