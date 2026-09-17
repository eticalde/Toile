#![allow(missing_docs, reason = "a test crate publishes no API surface")]

/// Draping the demo bodice over the reference adult Anny body must always
/// produce these bits.
///
/// The body is `toile_engine::golden`'s own `REFERENCE`, the one
/// `anny_mesh_hash` and `anny_sdf_hash` already pin, so a body that moves
/// shows in all three and a drape that moves on its own shows only here.
///
/// CI runs it on macOS ARM and Linux x86 against the same constant, the way
/// the sphere's own drape golden is run.
#[test]
#[ignore = "release-only golden: cargo test --release -- --ignored"]
fn the_bodice_drapes_on_the_reference_body_to_a_fixed_value() {
    assert_eq!(
        toile_engine::golden::drape_on_anny_hash(),
        0x51fc_b075_8b69_1033,
        "the drape on the body changed bits: the body moved, the bake moved, \
         the height a garment is let go from moved, or the solver did. If on \
         purpose, regenerate with `cargo run --release -p toile-cli -- \
         drape-anny`, re-pin the constant here in that same commit and say why \
         there; `git log -S` on the literal it replaces finds every earlier move"
    );
}

/// The eternal golden does not move because a body exists beside it.
///
/// The sphere's scene is built and hashed in the very process that has just
/// baked an adult body and draped the same panel over it, which is the one
/// arrangement in which a shared fixture quietly gone wrong would show.
#[test]
#[ignore = "release-only golden: cargo test --release -- --ignored"]
fn switching_back_to_the_sphere_reproduces_the_eternal_golden() {
    assert_eq!(
        toile_engine::golden::drape_on_anny_hash(),
        0x51fc_b075_8b69_1033
    );
    assert_eq!(
        toile_engine::golden::drape_bodice_hash(),
        0x534d_d0e5_200e_8e4a,
        "the sphere is the physics reference: putting a body beside it moves \
         nothing about the scene the golden pins"
    );
}
