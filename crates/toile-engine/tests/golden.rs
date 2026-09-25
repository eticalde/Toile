#![allow(missing_docs, reason = "a test crate publishes no API surface")]

/// Drape, edit, re-drape must always produce these bits.
///
/// CI runs this in release on macOS ARM and Linux x86 against the same
/// constant. Both passing is what makes the f32 solver bit-identical across
/// architectures rather than merely deterministic on one.
///
/// A legitimate physics change moves the hash.
#[test]
#[ignore = "release-only golden: cargo test --release -- --ignored"]
fn drape_bodice_golden() {
    assert_eq!(
        toile_engine::golden::drape_bodice_hash(),
        0x534d_d0e5_200e_8e4a,
        "the golden drape changed bits: either non-determinism crept in, or a \
         legitimate physics change needs the golden regenerated. If on purpose, \
         regenerate with `cargo run --release -p toile-cli -- drape`, re-pin the \
         constant here in that same commit and say why there; `git log -S` on \
         the literal it replaces finds every earlier move"
    );
}

/// A sewn garment on a body must always come to the same bits.
///
/// The only golden whose scene has a seam in it: the shipped block's two pieces
/// paired the way the studio pairs them, over the reference adult, with ground
/// under it, ten simulated seconds. It moves when the pairing moves, when the
/// sewing schedule moves, when the weightless phase moves and when the mesher
/// moves — none of which the other eight can see.
///
/// What it does not watch, measured and not assumed: no particle of this scene
/// ever ends a substep out past the band the field was baked to, so its hash is
/// the same either side of the contact solve's retreat along an overshooting
/// step. Reaching that far in wants a seam whose two sides were let go on
/// opposite sides of the body, which wants the studio's own placement. The
/// retreat is watched instead by `xpbd::contact`'s unit tests and by the
/// wide-hipped drape in `tests/seeding/extremes.rs`.
///
/// A legitimate physics change moves the hash.
#[test]
#[ignore = "release-only golden: cargo test --release -- --ignored"]
fn drape_sewn_golden() {
    assert_eq!(
        toile_engine::golden::drape_sewn_hash(),
        0xa9c8_cb4a_b906_1d1d,
        "the sewn drape changed bits: either non-determinism crept in, or a \
         legitimate change to sewing, placement or contact needs the golden \
         regenerated. If on purpose, regenerate with `cargo run --release -p \
         toile-cli -- drape-sewn`, re-pin the constant here in that same commit \
         and say why there"
    );
}
