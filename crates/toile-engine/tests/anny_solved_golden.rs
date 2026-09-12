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
/// instead (giving `0x4b89_4f27_006a_f5e1`); and again when `entrepierna`
/// and `largo_lateral` started measuring to the floor instead of the
/// ankle ring and the group's own settling test was fixed to watch the
/// `height` parameter itself rather than `estatura`'s already-close
/// achieved value, which used to call the group settled after a single
/// round even while the lengths it had just solved were about to go
/// stale. `largo_lateral`'s tied leg-height pair now correctly saturates
/// at its own bound on Toile's default tape: fully shortened, the model's
/// own leg-to-torso ratio still cannot reach a floor-based 104 cm at 178
/// cm of stature, which the ankle-based reading never revealed (giving
/// `0x4af5_aada_7f1b_67d4`).
///
/// It moved again once a girth was summed in the plane its ring was cut on
/// instead of in space. Every solved girth had been reading long by the
/// amount the phenotype buckled its ring out of plane, so the solver was
/// closing the Δ by shrinking the body underneath it; with the reading
/// honest, the same tape lands on a different lever vector and a
/// correspondingly different mesh. `pecho_alto`'s ring also moved two
/// centimetres down off the armpit apex, but that row has no lever, so it
/// changed what is reported and not what is solved (giving
/// `0x035d_560e_dafa_a8ef`).
///
/// It moved again because `cintura`'s ring dropped a template centimetre off
/// the narrowest section to the waistline a garment actually sits on. Two
/// levers were jammed against opposite stops by that misplacement on every
/// phenotype — `napetowaist-dist` at `+1` reaching for a longer
/// `largo_espalda`, `waisttohip-dist` at `-1` reaching for a shorter
/// `altura_cadera` — and the drop is sized by where `altura_cadera` lands:
/// one template centimetre brings it inside tolerance of its dado, so the
/// row stops reporting a model limit, and `waisttohip-dist` comes off its
/// stop to `-0.48` to let it. The waist girth the solver is chasing sits
/// on a fuller section now, so `waist-circ` and the trunk levers that couple
/// to it settle at different values and the mesh follows (giving
/// `0x7d59_f877_a2c3_05b1`).
///
/// It moves here because `largo_lateral` stopped being a waist height and
/// became the waist-to-ankle length PLAN-002's decision 3-bis asks for,
/// against a new ankle landmark (`toile_anny::asset::RingId::AnkleJoint`).
/// None of what follows is arithmetic on a frozen body: the leg-height pair
/// reacted, and took the rest with it. The pair had
/// been jammed at its short stop chasing a 104 cm reading that included the
/// whole foot and could never be reached, and once the reading dropped by
/// the ankle's own 7 cm the pair came off that stop and stretched the legs
/// back out until the length was right. `largo_lateral` closes (+3.82 TOPE
/// to −0.10) and `muneca`'s own tope clears with it, but the rows that ride
/// those levers without owning one pay for it: `entrepierna` goes +0.90 to
/// +2.85 and `tiro` +1.95 to +2.89, because the crotch rose with the legs,
/// and `cabeza` goes +0.60 to −1.81 since stature is held at 178 cm by
/// shrinking everything the legs are not. That is the honest reading of
/// this body: at a waist 110.7 cm off the floor and an ankle at 6.8, its
/// own crotch sits at 80.9 rather than the tape's 78, and only a row with a
/// lever can argue.
///
/// Take the new value from this assertion and update the constant in the
/// same commit, saying why.
#[test]
fn solving_the_default_tape_hashes_to_a_fixed_value() {
    assert_eq!(
        toile_engine::golden::anny_solved_hash(),
        0x000b_cd63_66e3_93c3,
        "the default tape's solved levers or mesh changed bits: the solver \
         changed on purpose, a ring it solves against moved, or a \
         dependency drifted under it"
    );
}
