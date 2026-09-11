#![allow(missing_docs, reason = "a test crate publishes no API surface")]

/// Loading a reference adult Anny body and hashing its mesh must always
/// produce this value.
///
/// The reference is [`toile_engine::golden::anny_mesh_hash`]'s own
/// phenotype: male (`gender: 0.0`), age parameter `0.8`, and every other
/// field at `0.5` — see that function's doc for why those particular
/// values. This is the second vertical slice's golden: the first pinned the
/// bare neutral template (`0xb3f8_8dd8_6abc_c96f`); this one moves
/// deliberately once the phenotype actually drives the mesh, and pins a
/// body distinct from every symmetric default so a sign error in an
/// interpolation direction cannot hide.
///
/// It runs in the ordinary suite because decoding the embedded asset,
/// applying the phenotype's weighted deltas and computing normals all cost
/// low milliseconds: no solver, no GPU. CI runs it on macOS ARM and Linux
/// x86 against the same constant, which is what makes it bit-identical
/// across architectures rather than merely repeatable on one — every step
/// stays in the `+ - * / sqrt` regime and never calls a transcendental.
///
/// A re-bake of `crates/toile-anny/assets/body.bin` (source data changed,
/// or the baker's station, triangulation or quantization rule changed), a
/// change to the reference phenotype above, or a change to the evaluator's
/// math all move this hash. Take the new value from this assertion and
/// update the constant in the same commit, saying why.
#[test]
fn the_anny_body_hashes_to_a_fixed_value() {
    assert_eq!(
        toile_engine::golden::anny_mesh_hash(),
        0x6254_3e96_3147_07eb,
        "the Anny body's mesh changed bits: the asset was re-baked on \
         purpose, the reference phenotype changed, or a dependency drifted \
         under it"
    );
}
