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
