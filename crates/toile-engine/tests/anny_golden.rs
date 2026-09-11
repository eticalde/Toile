#![allow(missing_docs, reason = "a test crate publishes no API surface")]

/// Loading the neutral Anny body and hashing its mesh must always produce
/// this value.
///
/// It runs in the ordinary suite because decoding the embedded asset and
/// computing normals both cost microseconds: no bake, no solver, no GPU. CI
/// runs it on macOS ARM and Linux x86 against the same constant, which is
/// what makes it bit-identical across architectures rather than merely
/// repeatable on one — normals are computed in the `+ - * / sqrt` regime and
/// never call a transcendental, and the positions themselves are read
/// straight out of the asset with no floating-point arithmetic at all.
///
/// A re-bake of `crates/toile-anny/assets/body.bin` (source data changed, or
/// the baker's station or triangulation rule changed) moves this hash, and so
/// would a change to how normals are computed. Take the new value from this
/// assertion and update the constant in the same commit, saying why.
#[test]
fn the_anny_body_hashes_to_a_fixed_value() {
    assert_eq!(
        toile_engine::golden::anny_mesh_hash(),
        0xb3f8_8dd8_6abc_c96f,
        "the Anny body's mesh changed bits: the asset was re-baked on \
         purpose, or a dependency drifted under it"
    );
}
