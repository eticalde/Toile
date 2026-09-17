#![allow(missing_docs, reason = "a test crate publishes no API surface")]

/// Baking the reference adult Anny body into its collision field must always
/// produce this grid.
///
/// The reference phenotype is `toile_engine::golden`'s own `REFERENCE`, the
/// same body `anny_mesh_hash` pins, so a mesh change shows in both and a bake
/// change shows only here.
///
/// CI runs it on macOS ARM and Linux x86 against the same constant. The bake
/// is `+ − × ÷ sqrt` and `libm::acos`, which is the same code on both, so
/// both passing is what makes the field bit-identical across architectures
/// rather than merely repeatable on one.
///
/// A pocket under the jaw is why this value is not the one before it: the
/// flood fill used to read "unreachable from the box's rim" as "inside the
/// body", and 232 samples of air it had sealed off read as flesh.
#[test]
#[ignore = "release-only golden: cargo test --release -- --ignored"]
fn the_reference_body_sdf_hashes_to_a_fixed_value() {
    assert_eq!(
        toile_engine::golden::anny_sdf_hash(),
        0x65c4_71da_b2e2_4930,
        "the baked body field changed bits: the mesh moved, the cell or band \
         changed, the bake's arithmetic changed, or `libm` drifted under it. \
         If on purpose, bump `body::bake::BAKE_VERSION`, re-pin the constant \
         here in that same commit and say why there; `git log -S` on the \
         literal it replaces finds every earlier move"
    );
}
