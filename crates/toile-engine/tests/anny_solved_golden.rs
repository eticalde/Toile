#![allow(missing_docs, reason = "a test crate publishes no API surface")]

/// Solving every lever against Toile's own default tape
/// (`toile_engine::body::default_measures`) for the default phenotype must
/// always produce this lever vector and this resulting mesh, hashed
/// together.
///
/// This is the fourth vertical slice's golden: one level above
/// `anny_measures_golden.rs`, which only pins where the rings land — this
/// one additionally pins what the solver does with them. It moves when the
/// secant step, the fixed solve order, the tolerance, or a ring a lever is
/// solved against changes; it stays put across everything that keeps
/// producing the same lever vector, which is the actual proof the solve is
/// deterministic rather than merely today's numbers happening to agree.
///
/// It has moved once since it was first pinned at `0xa113_0836_d2c4_2950`:
/// `estatura` and the four stature-affecting lengths used to be solved in
/// a single pass, which left Toile's own default tape settling 5.43 cm
/// too tall (183.43 cm against a 178 cm dado) because the leg-height
/// levers `largo_lateral` drives move stature after the height phenotype
/// was already solved for it. That group now iterates — revisiting
/// height after the lengths run, up to a small fixed round cap, stature
/// treated as effectively hard the same way `docs/anny.html`'s plan
/// treats it — so `estatura` settles within its own tight tolerance and
/// `largo_lateral` (and `entrepierna`, which has no lever of its own but
/// shares that lever pair) carry whatever residual the coupling leaves
/// instead.
///
/// Take the new value from this assertion and update the constant in the
/// same commit, saying why.
#[test]
fn solving_the_default_tape_hashes_to_a_fixed_value() {
    assert_eq!(
        toile_engine::golden::anny_solved_hash(),
        0x4b89_4f27_006a_f5e1,
        "the default tape's solved levers or mesh changed bits: the solver \
         changed on purpose, a ring it solves against moved, or a \
         dependency drifted under it"
    );
}
