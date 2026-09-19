use toile_sim::xpbd::DistanceConstraints;

use super::pipeline::COMPLIANCE;

/// The strength an elastic that merely holds its ratio is written at.
///
/// CLO3D's own default, kept at the same number because a value a person has
/// in their fingers from one tool should mean the same thing here.
pub const HOLDS_ITS_RATIO: f64 = 10.0;

/// One edge an elastic holds, as the product's combined constraints address
/// it.
///
/// No new constraint type: a stretch held in is the edges it runs over,
/// resting shorter than they were drawn and pulling harder. Everything the
/// solver needs of an elastic is already in `DistanceConstraints`, which has
/// carried a rest length and a compliance per edge since it was written.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Held {
    /// Where the edge sits among the product's constraints.
    pub edge: usize,
    /// What the drawn rest length is multiplied by.
    pub ratio: f32,
    /// What compliance the edge runs at.
    pub compliance: f32,
}

/// The compliance an elastic of `strength` runs its edges at.
///
/// Strength is a multiple of the cloth's own stiffness, and that is the whole
/// of the mapping: one is an elastic no firmer than the fabric it is set
/// into, ten is ten times firmer, and a tenth is a length of shirring that
/// gives before the cloth does. A person reasons about it without knowing
/// what a compliance is, and it says something true whatever `COMPLIANCE`
/// itself is later tuned to — a number in newtons per metre would have to be
/// re-chosen the day the cloth changed.
///
/// Ten is where CLO3D puts its default and what it means there — "holds its
/// ratio against outside influence" — so the reading carries over.
pub fn compliance_of(strength: f64) -> f32 {
    (f64::from(COMPLIANCE) / strength) as f32
}

/// Writes what the elastics hold onto a product's constraints.
///
/// A product with no elastic is handed back untouched: not re-derived, not
/// rounded back to where it was, not written over with the values it already
/// had — the loop does not run. That is what lets every drape golden go
/// through this path unmoved.
pub fn hold(cons: &mut DistanceConstraints, held: &[Held]) {
    for one in held {
        if let Some(rest) = cons.rest.get_mut(one.edge) {
            *rest *= one.ratio;
        }
        if let Some(compliance) = cons.compliance.get_mut(one.edge) {
            *compliance = one.compliance;
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "an edge nothing held carries the very bits it was given"
    )]

    use super::super::pipeline::ShapePipeline;
    use super::super::product::combine_constraints;
    use super::*;

    fn pipeline() -> ShapePipeline {
        let rectangle = [[0.0, 0.0], [0.30, 0.0], [0.30, 0.20], [0.0, 0.20]];
        ShapePipeline::build(&rectangle, 16, 0.01).expect("the rectangle is finite")
    }

    /// The reduction the drape goldens stand on: with nothing held, the
    /// constraints are the constraints of today, bit for bit.
    #[test]
    fn a_product_with_no_elastic_is_the_product_of_today() {
        let pipe = pipeline();
        let plain = combine_constraints(&[&pipe], COMPLIANCE);
        let mut held = combine_constraints(&[&pipe], COMPLIANCE);
        hold(&mut held, &[]);
        assert_eq!(held.rest, plain.rest);
        assert_eq!(held.compliance, plain.compliance);
    }

    /// And an edge that is held rests shorter and pulls harder, while every
    /// other edge of the same product is left exactly as it was.
    #[test]
    fn a_held_edge_rests_shorter_and_its_neighbours_do_not() {
        let pipe = pipeline();
        let plain = combine_constraints(&[&pipe], COMPLIANCE);
        let mut cons = combine_constraints(&[&pipe], COMPLIANCE);
        hold(
            &mut cons,
            &[Held {
                edge: 3,
                ratio: 0.85,
                compliance: compliance_of(HOLDS_ITS_RATIO),
            }],
        );
        assert_eq!(cons.rest[3], plain.rest[3] * 0.85);
        assert_eq!(cons.compliance[3], COMPLIANCE / 10.0);
        for e in (0..cons.len()).filter(|&e| e != 3) {
            assert_eq!(cons.rest[e], plain.rest[e]);
            assert_eq!(cons.compliance[e], plain.compliance[e]);
        }
    }

    /// Strength reads as a multiple of the cloth's own stiffness, in both
    /// directions from it.
    #[test]
    fn strength_is_a_multiple_of_the_cloths_own_stiffness() {
        assert_eq!(compliance_of(1.0), COMPLIANCE);
        assert_eq!(compliance_of(HOLDS_ITS_RATIO), COMPLIANCE / 10.0);
        assert!(compliance_of(0.1) > COMPLIANCE, "a slack elastic gives");
    }
}
