#![allow(missing_docs, reason = "a test crate publishes no API surface")]

/// Solving every lever against Toile's own default tape
/// (`toile_engine::body::default_measures`) for the default phenotype must
/// always produce this lever vector and this resulting mesh, hashed
/// together.
///
/// It moves when the secant step, the fixed solve order, the tolerance, or
/// a ring a lever is solved against changes. It stays put across everything
/// else that still produces the same lever vector, and that is the point:
/// holding still is the proof the solve is deterministic, rather than
/// today's numbers happening to agree.
#[test]
fn solving_the_default_tape_hashes_to_a_fixed_value() {
    assert_eq!(
        toile_engine::golden::anny_solved_hash(),
        0x000b_cd63_66e3_93c3,
        "the default tape's solved levers or mesh changed bits: the solver \
         changed on purpose, a ring it solves against moved, or a \
         dependency drifted under it. If on purpose, re-pin the constant \
         here in that same commit and say why there; `git log -S` on the \
         literal it replaces finds every earlier move"
    );
}
