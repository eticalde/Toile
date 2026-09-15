#![allow(missing_docs, reason = "a test crate publishes no API surface")]

/// Measuring the reference adult Anny body (the same phenotype
/// `anny_golden.rs` pins the mesh of) must always produce these 20
/// centimetre values, hashed together.
///
/// It pins where the measurement rings and paths land, and which line on the
/// skin each length follows, which no mesh golden can: a ring or a path
/// moving — a different band, a different plane, a different arc kept —
/// moves this hash while leaving both mesh hashes untouched, since measuring
/// only ever reads positions. It also moves whenever a mesh golden does, the
/// body under the rings and paths having changed.
#[test]
fn the_anny_measures_hash_to_a_fixed_value() {
    assert_eq!(
        toile_engine::golden::anny_measures_hash(),
        0x9793_2614_954b_0935,
        "the Anny body's measured values changed bits: a ring or path moved on \
         purpose, or a dependency drifted under it. If on purpose, re-pin \
         the constant here in that same commit and say why there; \
         `git log -S` on the literal it replaces finds every earlier move"
    );
}
