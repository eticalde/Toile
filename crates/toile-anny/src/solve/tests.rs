use super::*;
use crate::phenotype::LEVERS;

fn lever_id(label: &str) -> u8 {
    LEVERS
        .iter()
        .position(|&l| l == label)
        .expect("known lever label") as u8
}

const ZERO: [f64; 20] = [0.0; 20];

#[test]
fn solving_a_reachable_waist_target_lands_within_tolerance() {
    let phenotype = Phenotype::default();
    let current = measure::measure(&crate::body_mesh(&phenotype, &ZERO).positions).waist;
    let target = f64::from(current) + 5.0;

    let solved = solve_girth(&phenotype, &ZERO, &[lever_id("waist-circ")], target, |m| {
        m.waist
    });
    assert!(
        !solved.saturated,
        "a 5 cm move should be well within the model's own range"
    );
    assert!(
        (solved.achieved_cm - target).abs() <= TOLERANCE_CM,
        "achieved {} vs target {target}",
        solved.achieved_cm
    );
    assert!(solved.value.abs() <= 1.0);
}

#[test]
fn a_target_beyond_the_levers_reach_saturates_without_diverging() {
    let phenotype = Phenotype::default();
    let solved = solve_girth(&phenotype, &ZERO, &[lever_id("waist-circ")], 1000.0, |m| {
        m.waist
    });
    assert!(solved.saturated, "1000 cm cannot be a reachable waist");
    assert!(
        (solved.value - 1.0).abs() < 1.0e-9,
        "should clamp to the incr bound, got {}",
        solved.value
    );
    assert!(solved.achieved_cm.is_finite() && solved.achieved_cm < 300.0);
}

#[test]
fn solving_is_idempotent() {
    let phenotype = Phenotype::default();
    let current = measure::measure(&crate::body_mesh(&phenotype, &ZERO).positions).waist;
    let target = f64::from(current) - 3.0;
    let a = solve_girth(&phenotype, &ZERO, &[lever_id("waist-circ")], target, |m| {
        m.waist
    });
    let b = solve_girth(&phenotype, &ZERO, &[lever_id("waist-circ")], target, |m| {
        m.waist
    });
    assert_eq!(a.value.to_bits(), b.value.to_bits());
    assert_eq!(a.achieved_cm.to_bits(), b.achieved_cm.to_bits());
}

#[test]
fn a_tied_pair_moves_both_levers_together() {
    let phenotype = Phenotype::default();
    let current = measure::measure(&crate::body_mesh(&phenotype, &ZERO).positions).outseam;
    let target = f64::from(current) + 4.0;
    let ids = [lever_id("upperleg-height"), lever_id("lowerleg-height")];
    let solved = solve_girth(&phenotype, &ZERO, &ids, target, |m| m.outseam);
    assert!(!solved.saturated);
    assert!((solved.achieved_cm - target).abs() <= TOLERANCE_CM);
}

#[test]
fn solving_estatura_lands_within_tolerance_and_is_idempotent() {
    let phenotype = Phenotype::default();
    let current = measure::measure(&crate::body_mesh(&phenotype, &ZERO).positions).height;
    let target = f64::from(current) + 5.0;
    let a = solve_height(&phenotype, &ZERO, target);
    assert!(!a.saturated);
    assert!((a.achieved_cm - target).abs() <= TOLERANCE_CM);
    let b = solve_height(&phenotype, &ZERO, target);
    assert_eq!(a.value.to_bits(), b.value.to_bits());
}

#[test]
fn solving_height_beyond_reach_saturates() {
    let phenotype = Phenotype::default();
    let solved = solve_height(&phenotype, &ZERO, 1000.0);
    assert!(solved.saturated);
    assert!((solved.value - 1.0).abs() < 1.0e-9);
}
