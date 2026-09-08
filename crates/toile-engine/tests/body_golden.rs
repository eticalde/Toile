#![allow(missing_docs, reason = "a test crate publishes no API surface")]

/// Lofting the reference mannequin and hashing its mesh must always produce
/// these bits.
///
/// It runs in the ordinary suite because it costs microseconds: no solver, no
/// GPU, no wall clock. CI runs it on macOS ARM and Linux x86 against the same
/// constant, which is what makes the loft bit-identical across architectures
/// rather than merely repeatable on one — the reason the generator stays inside
/// the `+ - * / sqrt` regime and never calls a transcendental.
///
/// A deliberate change to the body's shape, its region constants or its
/// resolution moves the hash. Take the new value from this assertion and update
/// the constant in the same commit, saying why in the message.
///
/// The current value pins the full body in the A-pose: head, neck, shoulders
/// and arms, split-depth blended sections, and PCHIP profiles between the
/// landmarks — a deliberate shape change from the legs-and-dome dummy.
#[test]
fn the_reference_body_hashes_to_a_fixed_value() {
    assert_eq!(
        toile_engine::golden::body_mesh_hash(),
        0x5f47_f494_d7a6_4138,
        "the body mesh changed bits: the loft moved on purpose, or a \
         dependency drifted under it"
    );
}
