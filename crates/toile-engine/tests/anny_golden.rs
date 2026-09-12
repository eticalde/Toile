#![allow(missing_docs, reason = "a test crate publishes no API surface")]

/// Loading a reference adult Anny body and hashing its mesh must always
/// produce this value.
///
/// The reference phenotype is `toile_engine::golden`'s own `REFERENCE`:
/// male, age parameter `0.8`, every other input at Anny's neutral `0.5` —
/// see that constant for why it is not the symmetric default. A re-bake of
/// `crates/toile-anny/assets/body.bin`, a change to that phenotype, or a
/// change to the evaluator's math all move this hash.
///
/// CI runs it on macOS ARM and Linux x86 against the same constant, which
/// is what makes it bit-identical across architectures rather than merely
/// repeatable on one.
#[test]
fn the_anny_body_hashes_to_a_fixed_value() {
    assert_eq!(
        toile_engine::golden::anny_mesh_hash(),
        0x6254_3e96_3147_07eb,
        "the Anny body's mesh changed bits: the asset was re-baked on \
         purpose, the reference phenotype changed, or a dependency drifted \
         under it. If on purpose, re-pin the constant here in that same \
         commit and say why there; `git log -S` on the literal it replaces \
         finds every earlier move"
    );
}
