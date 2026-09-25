use toile_doc::Elastic;
use toile_sim::xpbd::DistanceConstraints;

/// The strength an elastic that merely holds its ratio is written at.
///
/// CLO3D's own default, kept at the same number because a value a person has
/// in their fingers from one tool should mean the same thing here.
pub const HOLDS_ITS_RATIO: f64 = 10.0;

/// The compliance an elastic of strength one runs at.
///
/// An elastic's own scale, and not a multiple of the cloth's. Woven cloth
/// gives a couple of per cent under load and a length of elastic tape gives a
/// hundred, so a band is the slacker of the two by orders of magnitude — and a
/// rail written in multiples of the cloth's stiffness spent the whole of its
/// length where one pass at dt = 1/600 s cannot tell one end from the other:
/// measured across the shipped 0.1×–50×, the settled hem moved 0.20 mm.
///
/// A thousand times the cloth's own compliance puts the shipped rail across
/// the span the solver resolves. Measured on the laced ring
/// `a_held_rings_stiffness_is_felt_once_it_is_swept_again` settles, the four
/// compliances the rail writes come to four different lengths; measured on the
/// whole skirt three simulated seconds in, the waistband stands at 120 % of the
/// length it is held to at the default of ten and at 143 % at a strength of a
/// tenth, which is the slack end of the rail.
const ELASTIC: f64 = 1.0e-5;

/// Extra solver sweeps the edges an elastic holds get.
///
/// A budget on a measured curve, and not a plateau. Read on the seeded skirt
/// three simulated seconds in, the waistband stands at 149 % of the length it
/// is held to with no extra sweep at all, 120 % at sixty-four, 113 % at two
/// hundred and fifty-six and 109 % at 1024: sixteen times the work past here
/// buys eleven points. Sixty-four sweeps of the couple of hundred edges a band
/// runs over is about a fifth of one sweep of the garment's own sixty-seven
/// thousand, and 1024 is about three whole sweeps more every substep.
///
/// What the last eleven points are worth is not nothing, and the number is
/// the owner's to move: on that one skirt and body the band swept 1024 times
/// was still above the hip twenty simulated seconds in, where at sixty-four
/// and at 256 it had travelled down the legs.
///
/// `a_waistband_grips_the_waist_and_still_travels_down_the_body` in
/// `tests/seeding/grip.rs` is the test that fails when this is zero.
pub const HOLDS_PASSES: u32 = 64;

/// One stretch of cloth an elastic holds in, as the product's combined
/// constraints address it.
///
/// No new constraint type: a stretch held in is the edges it runs over,
/// resting shorter than they were drawn, pulling at the elastic's own
/// compliance rather than the cloth's, and swept again so that they are not
/// outvoted by the cloth around them. All three are things
/// `DistanceConstraints` already carries.
#[derive(Debug, Clone, PartialEq)]
pub struct Held {
    /// Where the stretch's edges sit among the product's constraints.
    pub edges: Vec<usize>,
    /// What the drawn rest length is multiplied by.
    pub ratio: f32,
    /// What compliance the stretch runs at.
    pub compliance: f32,
}

/// The compliance an elastic of `strength` runs at.
///
/// Strength is a multiple of [`ELASTIC`], the stiffness of a band of strength
/// one, and that is the whole of the mapping: ten is ten times firmer than
/// that band, and a tenth is a length of shirring that gives ten times sooner.
/// A person reasons about it without knowing what a compliance is, and it
/// stays true whatever the cloth is later tuned to.
///
/// Ten is where CLO3D puts its default and what it means there — "holds its
/// ratio against outside influence" — so the reading carries over, and here it
/// is measured: `tests/seeding/grip.rs` drapes one skirt at the rail's slack
/// end, its knee and its default, and reads what the band came to.
///
/// The slackest answer is the slackest band the document admits,
/// [`Elastic::MIN_STRENGTH`], whatever weaker number a caller gives: a held
/// edge runs at this in place of the cloth's own compliance, and past what an
/// `f32` spells every correction on it is exactly zero — no band, no cloth.
pub fn compliance_of(strength: f64) -> f32 {
    (ELASTIC / strength.max(Elastic::MIN_STRENGTH)) as f32
}

/// Writes what the elastics hold onto a product's constraints.
///
/// A product with no elastic is handed back untouched: not re-derived, not
/// rounded back to where it was, not written over with the values it already
/// had, and not walked over once — this returns before it allocates. That is
/// what lets every drape golden go through this path unmoved. The edges an
/// elastic does hold are also named for the extra sweeps: an outvoted band
/// holds nothing, and `DistanceConstraints::held` is where that is argued.
///
/// Two elastics over one stretch hold it once, at the tighter ratio and the
/// firmer compliance, and this is the one place that rule is written. An edge
/// carries one rest length because it is one length of cloth, and two bands
/// round one waist pull it to the tighter of the two, never to the product:
/// multiplying would make the same waistband put on twice ask for 0.72 of the
/// drawn length instead of the 0.85 either of them asks for. The placement
/// reads the same cloth the same way — `Session::elastics` widens a piece's
/// band once per piece, not once per elastic.
pub fn hold(cons: &mut DistanceConstraints, held: &[Held]) {
    if held.is_empty() {
        return;
    }
    let mut tightest: Vec<Option<(f32, f32)>> = vec![None; cons.len()];
    for one in held {
        for &edge in &one.edges {
            let Some(slot) = tightest.get_mut(edge) else {
                continue;
            };
            *slot = Some(match *slot {
                Some((tight, firm)) => (tight.min(one.ratio), firm.min(one.compliance)),
                None => (one.ratio, one.compliance),
            });
        }
    }
    for (edge, tight) in tightest.iter().enumerate() {
        if let Some((ratio, compliance)) = *tight {
            cons.rest[edge] *= ratio;
            cons.compliance[edge] = compliance;
            cons.held.push(edge as u32);
        }
    }
    if !cons.held.is_empty() {
        cons.held_passes = HOLDS_PASSES;
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "an edge nothing held carries the very bits it was given"
    )]

    use super::super::pipeline::{COMPLIANCE, ShapePipeline};
    use super::super::product::combine_constraints;
    use super::*;

    fn pipeline() -> ShapePipeline {
        let rectangle = [[0.0, 0.0], [0.30, 0.0], [0.30, 0.20], [0.0, 0.20]];
        ShapePipeline::build(&rectangle, 16, 0.01).expect("the rectangle is finite")
    }

    /// A stretch over one edge of the mesh, with no walk: enough to read what
    /// `hold` writes per edge without a chain to hold to a length.
    fn stretch(edge: usize, ratio: f32, strength: f64) -> Held {
        Held {
            edges: vec![edge],
            ratio,
            compliance: compliance_of(strength),
        }
    }

    /// The reduction the drape goldens stand on: with nothing held, the
    /// constraints are the constraints of today, bit for bit, and no band.
    #[test]
    fn a_product_with_no_elastic_is_the_product_of_today() {
        let pipe = pipeline();
        let plain = combine_constraints(&[&pipe], COMPLIANCE);
        let mut held = combine_constraints(&[&pipe], COMPLIANCE);
        hold(&mut held, &[]);
        assert_eq!(held.rest, plain.rest);
        assert_eq!(held.compliance, plain.compliance);
        assert!(held.held.is_empty(), "and nothing is swept again");
        assert_eq!(held.held_passes, 0);
    }

    /// And an edge that is held rests shorter and pulls at the elastic's own
    /// compliance, while every other edge of the same product is left exactly
    /// as it was.
    #[test]
    fn a_held_edge_rests_shorter_and_its_neighbours_do_not() {
        let pipe = pipeline();
        let plain = combine_constraints(&[&pipe], COMPLIANCE);
        let mut cons = combine_constraints(&[&pipe], COMPLIANCE);
        hold(&mut cons, &[stretch(3, 0.85, HOLDS_ITS_RATIO)]);
        assert_eq!(cons.rest[3], plain.rest[3] * 0.85);
        assert_eq!(cons.compliance[3], compliance_of(HOLDS_ITS_RATIO));
        for e in (0..cons.len()).filter(|&e| e != 3) {
            assert_eq!(cons.rest[e], plain.rest[e]);
            assert_eq!(cons.compliance[e], plain.compliance[e]);
        }
    }

    /// A held edge is also named for the extra sweeps, and it is named once
    /// however many elastics run over it.
    #[test]
    fn a_held_edge_is_named_once_for_the_sweeps_that_let_it_carry_the_vote() {
        let pipe = pipeline();
        let mut cons = combine_constraints(&[&pipe], COMPLIANCE);
        hold(
            &mut cons,
            &[
                stretch(3, 0.85, HOLDS_ITS_RATIO),
                stretch(3, 0.55, HOLDS_ITS_RATIO),
                stretch(4, 0.85, HOLDS_ITS_RATIO),
            ],
        );
        assert_eq!(cons.held, vec![3, 4]);
        assert_eq!(cons.held_passes, HOLDS_PASSES);
        assert!(
            cons.held_passes > 0,
            "a band nobody sweeps again is outvoted"
        );
    }

    /// Strength reads as a multiple of a band of strength one, in both
    /// directions from it, and that band is slacker than the cloth.
    #[test]
    fn strength_is_a_multiple_of_a_bands_own_stiffness() {
        assert_eq!(compliance_of(1.0), ELASTIC as f32);
        assert_eq!(compliance_of(HOLDS_ITS_RATIO), (ELASTIC / 10.0) as f32);
        assert!(compliance_of(0.1) > compliance_of(1.0), "a slack one gives");
        assert!(
            compliance_of(HOLDS_ITS_RATIO) > COMPLIANCE,
            "an elastic that holds its ratio is still slacker than the weave"
        );
    }

    /// A strength under the document's floor runs as the floor.
    ///
    /// The document refuses one, so this is for whoever did not come through
    /// it. What is pinned is what the solver does with the answer at the step
    /// the drape runs at: `alpha` stays finite and the edge is still corrected,
    /// where the largest compliance an `f32` spells made `alpha` an infinity
    /// and the correction exactly zero — on an edge `hold` had already taken
    /// the cloth's own compliance from.
    #[test]
    fn a_strength_under_the_floor_runs_as_the_slackest_band_the_document_admits() {
        let slackest = compliance_of(Elastic::MIN_STRENGTH);
        let inv_dt2 = 600.0_f32 * 600.0;
        for strength in [1.0e-300, f64::MIN_POSITIVE, 1.0e-46, 0.0, -1.0, f64::NAN] {
            let compliance = compliance_of(strength);
            assert_eq!(compliance, slackest, "{strength}");
            let alpha = compliance * inv_dt2;
            assert!(alpha.is_finite(), "{strength}: alpha {alpha}");
            assert!(1.0 / (2.0 + alpha) > 0.0, "{strength}: no correction left");
        }
        assert!(
            (f32::MAX * inv_dt2).is_infinite(),
            "what the old ceiling did"
        );
        for strength in [0.1, 1.0, HOLDS_ITS_RATIO, 50.0] {
            assert!(compliance_of(strength) < slackest, "{strength}");
        }
    }

    /// Two elastics over one stretch hold it once, at the tighter ratio and
    /// the firmer compliance — never at the product of the two, which would
    /// have the same waistband put on twice pull to 0.7225 of the drawn
    /// length.
    #[test]
    fn two_elastics_over_one_edge_hold_it_once_at_the_tighter_of_the_two() {
        let pipe = pipeline();
        let plain = combine_constraints(&[&pipe], COMPLIANCE);
        let held = |bands: &[Held]| {
            let mut cons = combine_constraints(&[&pipe], COMPLIANCE);
            hold(&mut cons, bands);
            cons
        };

        let twice = held(&[
            stretch(3, 0.85, HOLDS_ITS_RATIO),
            stretch(3, 0.85, HOLDS_ITS_RATIO),
        ]);
        assert_eq!(twice.rest[3], plain.rest[3] * 0.85);
        assert_eq!(twice.compliance[3], compliance_of(HOLDS_ITS_RATIO));

        let mixed = held(&[stretch(3, 0.85, 1.0), stretch(3, 0.55, HOLDS_ITS_RATIO)]);
        assert_eq!(mixed.rest[3], plain.rest[3] * 0.55);
        assert_eq!(mixed.compliance[3], compliance_of(HOLDS_ITS_RATIO));

        // And the order the arena happened to hand them over in decides
        // nothing: one stretch of cloth has one answer.
        let swapped = held(&[stretch(3, 0.55, HOLDS_ITS_RATIO), stretch(3, 0.85, 1.0)]);
        assert_eq!(swapped.rest, mixed.rest);
        assert_eq!(swapped.compliance, mixed.compliance);
    }
}
