use super::*;
use crate::body::{NO_LEVERS, default_measures};

/// Solving the same measure set and phenotype twice must always produce
/// the same lever vector, bit for bit: [`solve_anny`]'s own doc promises
/// a fixed order with no data-dependent branch that could vary run to
/// run, and this is the end-to-end proof of that promise across the whole
/// 20-row solve, not just the single-lever secant `toile_anny::solve`
/// already covers.
#[test]
fn solving_the_default_tape_twice_is_bit_for_bit_identical() {
    let phenotype = Phenotype::default();
    let measures = default_measures();
    let a = solve_anny(&measures, &phenotype);
    let b = solve_anny(&measures, &phenotype);
    assert_eq!(a.levers.map(f64::to_bits), b.levers.map(f64::to_bits));
    for name in MeasureSet::CATALOGUE {
        let ra = a.rows.get(name).expect("every catalogue name has a row");
        let rb = b.rows.get(name).expect("every catalogue name has a row");
        assert_eq!(ra.medido_cm.to_bits(), rb.medido_cm.to_bits());
        assert_eq!(ra.saturated, rb.saturated);
    }
}

/// A full solve only ever moves vertex positions by summing more lever
/// deltas in: the mesh it produces keeps the same triangle indices and
/// vertex count as the zero-lever body at the same (solved) phenotype,
/// and the round-2 structural invariant — the hip sitting strictly
/// above the fork, so `altura_cadera` is always shorter than `tiro` —
/// survives being stretched by whatever a real tape actually asks for.
#[test]
fn a_full_solve_keeps_topology_and_the_hip_above_the_fork() {
    let phenotype = Phenotype::default();
    let measures = default_measures();
    let solved = solve_anny(&measures, &phenotype);
    let solved_mesh = toile_anny::body_mesh(&solved.phenotype, &solved.levers);
    let unsolved_mesh = toile_anny::body_mesh(&solved.phenotype, &NO_LEVERS);

    assert_eq!(solved_mesh.indices, unsolved_mesh.indices);
    assert_eq!(solved_mesh.positions.len(), unsolved_mesh.positions.len());

    let m = toile_anny::measure::measure(&solved_mesh.positions);
    assert!(
        m.hip_drop < m.rise,
        "altura_cadera {} is not shorter than tiro {} after a full solve",
        m.hip_drop,
        m.rise
    );
}

/// The fix this test guards: before [`solve_stature_group`] revisited
/// height after the stature-affecting lengths ran, Toile's own default
/// tape (178 cm plus a 104 cm `largo_lateral`) settled at 183.43 cm — a
/// 5.43 cm miss nobody would accept against a tape measure. Stature must
/// now land within the group's own settle tolerance, with the residual,
/// if any, showing up on the length rows instead.
#[test]
fn estatura_lands_close_on_the_default_tape_even_though_it_used_to_miss_by_five_cm() {
    let phenotype = Phenotype::default();
    let measures = default_measures();
    let solved = solve_anny(&measures, &phenotype);
    let row = &solved.rows["estatura"];
    assert!(
        row.delta_cm.unwrap().abs() <= STATURE_SETTLED_CM,
        "estatura missed its dado by {:.2} cm after the stature group ran",
        row.delta_cm.unwrap()
    );
}

/// Feeding a phenotype's own reading back at itself as its tape is a
/// fixed point: every lever should stay at (or very near) zero and every
/// row's Δ should stay at (or very near) zero, proving the iterated
/// stature group does not overshoot or oscillate when there is nothing
/// to correct.
#[test]
fn a_self_consistent_tape_still_settles_with_near_zero_delta() {
    let phenotype = Phenotype::default();
    let mesh = toile_anny::body_mesh(&phenotype, &NO_LEVERS);
    let m = toile_anny::measure::measure(&mesh.positions);
    let self_measures = MeasureSet::new(
        "self",
        MeasureSet::CATALOGUE
            .iter()
            .map(|&name| (name, f64::from(extract(name, &m))))
            .collect::<Vec<_>>(),
    );
    let solved = solve_anny(&self_measures, &phenotype);
    for name in MeasureSet::CATALOGUE {
        let row = &solved.rows[name];
        assert!(
            row.delta_cm.unwrap().abs() < 0.1,
            "{name}: Δ {:.4} cm on a tape that is already this body's own reading",
            row.delta_cm.unwrap()
        );
        assert!(
            !row.saturated,
            "{name} should not saturate on its own reading"
        );
    }
}
