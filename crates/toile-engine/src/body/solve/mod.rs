use std::collections::BTreeMap;

use toile_anny::measure::Measures;
use toile_anny::phenotype::{LEVERS, Phenotype};
use toile_anny::solve;

use super::READINGS;
use crate::draft::MeasureSet;

#[cfg(test)]
mod tests;

/// One catalogue row's outcome after a full solve: what the body actually
/// measures there, how far that sits from the dado, and whether a lever
/// even exists to close that gap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolvedRow {
    /// What the solved body measures for this name, in centimetres.
    pub medido_cm: f64,
    /// `medido_cm` minus the dado, or `None` if the measure set never
    /// wrote this name down.
    pub delta_cm: Option<f64>,
    /// Whether this name maps to a `measure-*` lever at all. `tiro`,
    /// `pecho_alto`, `entrepierna` and `cabeza` do not — see
    /// [`lever_ids`]'s doc — so their row never closes Δ no matter how far
    /// it sits from the dado.
    pub has_lever: bool,
    /// Whether the lever that drives this name hit its own `±1` bound
    /// without reaching the dado within `toile_anny::solve::TOLERANCE_CM`
    /// — "tope del modelo": the target is outside what this body can
    /// become, not a solver that gave up early. Always `false` when
    /// `has_lever` is `false`.
    pub saturated: bool,
}

/// The result of solving every lever against a measure set.
pub struct AnnySolve {
    /// Same as the input phenotype except `height`, which the solver sets
    /// to whatever value makes `estatura` match (or come closest to) the
    /// dado.
    pub phenotype: Phenotype,
    /// The 20 lever values, in [`LEVERS`] order, every solved row's own
    /// pair (or pairs, for a tied name) among them.
    pub levers: [f64; 20],
    /// Every catalogue name's outcome, keyed by its own Spanish name.
    pub rows: BTreeMap<String, SolvedRow>,
}

fn lever_id(label: &str) -> u8 {
    LEVERS
        .iter()
        .position(|&l| l == label)
        .unwrap_or_else(|| panic!("no lever named `{label}`")) as u8
}

/// The lever (or, for `largo_lateral` and `brazo`, the tied pair of levers)
/// a catalogue name is solved through, in [`LEVERS`] order.
///
/// `None` for `tiro`, `pecho_alto`, `entrepierna` and `cabeza`: none of the
/// 20 labels [`LEVERS`] carries moves any of those four on its own, the head
/// because the baked asset holds no head-scaling lever at all. `estatura` is
/// handled separately, through the `height` phenotype input rather than a row
/// lever.
///
/// `tiro` is the near miss, and the reason it stays a miss: `waisttohip-dist`
/// does have authority over the rise, but `altura_cadera` claims that lever
/// first and spends it re-lengthening the span to hold its own dado. Cutting
/// the waist ring lower shortens the rise only while that lever is pinned
/// against its short stop, and gives it back the moment the lever comes off.
fn lever_ids(name: &str) -> Option<Vec<u8>> {
    let one = |label: &str| Some(vec![lever_id(label)]);
    match name {
        "cintura" => one("waist-circ"),
        "cadera" => one("hips-circ"),
        "muslo" => one("thigh-circ"),
        "rodilla" => one("knee-circ"),
        "tobillo" => one("ankle-circ"),
        "cuello" => one("neck-circ"),
        "pecho" => one("bust-circ"),
        "bajo_pecho" => one("underbust-circ"),
        "brazo_contorno" => one("upperarm-circ"),
        "muneca" => one("wrist-circ"),
        "hombros" => one("shoulder-dist"),
        "altura_cadera" => one("waisttohip-dist"),
        "largo_espalda" => one("napetowaist-dist"),
        "largo_lateral" => Some(vec![
            lever_id("upperleg-height"),
            lever_id("lowerleg-height"),
        ]),
        "brazo" => Some(vec![
            lever_id("upperarm-length"),
            lever_id("lowerarm-length"),
        ]),
        _ => None,
    }
}

/// Reads the one field of a fresh [`Measures`] a catalogue name names,
/// through the same [`READINGS`] table `measured_anny` reports the *medido*
/// column from — one mapping, so the solver's target and the final report
/// cannot read different fields for the same name.
///
/// # Panics
/// If `name` is outside [`READINGS`]. A document may carry measurement names
/// the body cannot be read at — the catalogue guides and never governs, and
/// `MeasureSet` is what holds those — but a name reaching here is one this
/// crate has already claimed to measure.
fn extract(name: &str, m: &Measures) -> f32 {
    let row = READINGS
        .iter()
        .find(|row| row.name == name)
        .unwrap_or_else(|| panic!("`{name}` is not a catalogue name `extract` knows"));
    (row.read)(m)
}

/// Solves one catalogue name's lever(s) toward its dado, updating `levers`
/// and recording whether it saturated. Does nothing if the measure set
/// never wrote `name` down, or if `name` has no lever at all.
fn solve_one(
    levers: &mut [f64; 20],
    phenotype: &Phenotype,
    measures: &MeasureSet,
    name: &str,
    saturated: &mut BTreeMap<String, bool>,
) {
    let Some(target) = measures.get(name) else {
        return;
    };
    let Some(ids) = lever_ids(name) else {
        return;
    };
    let solved = solve::solve_girth(phenotype, levers, &ids, target, |m| extract(name, m));
    for &id in &ids {
        levers[id as usize] = solved.value;
    }
    saturated.insert(name.to_owned(), solved.saturated);
}

/// The four lengths that move stature: `largo_lateral`'s and `brazo`'s
/// tied leg- and arm-length levers push the mesh's own bounding box beyond
/// wherever the height solve last set it, and `largo_espalda` and
/// `altura_cadera` ride along in the same group since they are solved
/// against the same, still-settling phenotype.
const STATURE_LENGTHS: [&str; 4] = ["largo_espalda", "altura_cadera", "largo_lateral", "brazo"];

/// How many extra rounds to spend revisiting `estatura` after the
/// stature-affecting lengths have run, past the first seed solve. A small
/// fixed cap, not an unbounded loop: past this many rounds the last
/// round's values stand as final, and whatever the coupling could not
/// close is reported as the length rows' own Δ rather than chased forever.
/// A safety margin, not the expected count.
const STATURE_ROUNDS: u32 = 6;

/// How little the `height` phenotype input may move between two rounds
/// before the group counts as settled.
///
/// This tests the *parameter* itself, not `estatura`'s own achieved
/// centimetres: a single height solve nearly always lands within
/// `toile_anny::solve::TOLERANCE_CM` of its target by construction, so
/// testing the achieved value instead would call the group settled after
/// the very first round even while the lengths it just solved — against a
/// height that is about to move again — still have real work left to do.
const HEIGHT_SETTLED: f64 = 1.0e-4;

/// Solves `estatura` (the `height` phenotype input) together with
/// [`STATURE_LENGTHS`], since the leg-height and torso-length levers those
/// names drive move stature after the height solve has already set it.
///
/// Seeds `height` once against zero levers, then alternates solving the
/// four lengths and revisiting height, stopping as soon as `height` itself
/// has stopped moving between rounds ([`HEIGHT_SETTLED`]) or
/// [`STATURE_ROUNDS`] extra rounds have run. Every comparison that stop
/// makes is `+ - * /` over already-deterministic floats, so two calls with
/// the same inputs take the same path and land on the same bits: the early
/// exit does not reopen the non-determinism the fixed-iteration secant
/// itself avoids.
///
/// Stature wins when the two cannot both be satisfied: a length's own
/// secant re-targets its own catalogue value fresh every round, so it is
/// the *lengths* that carry whatever residual the coupling leaves once this
/// loop stops, not `estatura`.
fn solve_stature_group(
    current: &mut Phenotype,
    levers: &mut [f64; 20],
    measures: &MeasureSet,
    saturated: &mut BTreeMap<String, bool>,
) {
    let Some(target) = measures.get("estatura") else {
        for name in STATURE_LENGTHS {
            solve_one(levers, current, measures, name, saturated);
        }
        return;
    };

    let seed = solve::solve_height(current, levers, target);
    current.height = seed.value;
    saturated.insert("estatura".to_owned(), seed.saturated);

    let mut previous_height = current.height;
    for _round in 0..STATURE_ROUNDS {
        for name in STATURE_LENGTHS {
            solve_one(levers, current, measures, name, saturated);
        }
        let revisited = solve::solve_height(current, levers, target);
        current.height = revisited.value;
        saturated.insert("estatura".to_owned(), revisited.saturated);
        if (current.height - previous_height).abs() <= HEIGHT_SETTLED {
            break;
        }
        previous_height = current.height;
    }
}

/// Solves every catalogue lever against `measures`, in a fixed order.
///
/// `estatura` and the lengths that move it go first as one settling group
/// — see [`solve_stature_group`]; then the trunk girths in two sweeps
/// (`pecho`, `bajo_pecho`, `cintura`, `cadera`), because the waist lever
/// alone bleeds noticeably into the underbust and a single pass would
/// leave that coupling unresolved; then the limbs and head, which are
/// independent of everything above and of each other. `tiro`, `pecho_alto`,
/// `entrepierna` and `cabeza` have no lever ([`lever_ids`]) and are only
/// ever measured, never solved.
///
/// The order is fixed and always run in full: nothing skips a step or
/// changes how many times the trunk sweep repeats, and the one
/// data-dependent branch anywhere under here — [`solve_stature_group`]'s
/// early exit — is itself a deterministic function of deterministic floats.
/// So two calls with the same `measures` and `phenotype` always produce the
/// same `levers`, bit for bit.
pub fn solve_anny(measures: &MeasureSet, phenotype: &Phenotype) -> AnnySolve {
    let mut levers = [0.0f64; 20];
    let mut saturated: BTreeMap<String, bool> = BTreeMap::new();
    let mut current = *phenotype;

    solve_stature_group(&mut current, &mut levers, measures, &mut saturated);

    for _sweep in 0..2 {
        for name in ["pecho", "bajo_pecho", "cintura", "cadera"] {
            solve_one(&mut levers, &current, measures, name, &mut saturated);
        }
    }

    for name in [
        "cuello",
        "hombros",
        "muslo",
        "rodilla",
        "tobillo",
        "brazo_contorno",
        "muneca",
    ] {
        solve_one(&mut levers, &current, measures, name, &mut saturated);
    }

    let mesh = toile_anny::body_mesh(&current, &levers);
    let final_measures = toile_anny::measure::measure(&mesh.positions);

    let rows = MeasureSet::CATALOGUE
        .iter()
        .map(|&name| {
            let medido_cm = f64::from(extract(name, &final_measures));
            let dado = measures.get(name);
            let row = SolvedRow {
                medido_cm,
                delta_cm: dado.map(|d| medido_cm - d),
                has_lever: name == "estatura" || lever_ids(name).is_some(),
                saturated: saturated.get(name).copied().unwrap_or(false),
            };
            (name.to_owned(), row)
        })
        .collect();

    AnnySolve {
        phenotype: current,
        levers,
        rows,
    }
}
