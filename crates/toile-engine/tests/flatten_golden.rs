#![allow(missing_docs, reason = "a test crate publishes no API surface")]

/// Resolving the base block and cutting its curves into points must always
/// produce these bits.
///
/// It runs in the ordinary suite because it costs microseconds: no solver, no
/// mesh, no wall clock. CI runs it on macOS ARM and Linux x86 against the same
/// constant, which is what makes the flattening bit-identical across
/// architectures rather than merely repeatable on one. It sits one level below
/// the drape golden on purpose: a drift in `kurbo`, in the evaluator or in the
/// platform's floating point shows up here, named, instead of arriving as a
/// changed rest state nobody can read.
///
/// A deliberate change to the block's shape, its handles or its sample counts
/// moves the hash. Take the new value from this assertion and update the
/// constant in the same commit, saying why in the message.
#[test]
fn the_flattened_pants_front_hashes_to_a_fixed_value() {
    assert_eq!(
        toile_engine::golden::flatten_front_hash(),
        0x76e3_f9d1_f9ed_932b,
        "the flattened front changed bits: either the block moved on purpose, \
         or a dependency drifted under it"
    );
}
