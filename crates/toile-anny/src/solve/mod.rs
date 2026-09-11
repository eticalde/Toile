use crate::asset::RowKind;
use crate::measure::{self, Measures};
use crate::mesh::{accumulate_except, decoded, lever_weight, template_positions_f64};
use crate::phenotype::{self, Phenotype};

#[cfg(test)]
mod tests;

/// Every secant solve runs exactly this many iterations — deterministic
/// cost, no early exit on convergence (an early exit's step count would
/// itself depend on floating-point specifics, which is exactly the kind of
/// data-dependent, hard-to-reproduce branch this module avoids).
const ITERATIONS: u32 = 8;

/// How close a solved measurement must land to its target to count as
/// reached rather than saturated at the lever's own bound.
///
/// In centimetres; chosen from what the solver actually achieves on a
/// reachable target — see the crate's tests.
pub const TOLERANCE_CM: f64 = 0.05;

/// One lever's (or one tied pair's, or the height phenotype's) solved
/// state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Solved {
    /// The final value, clamped to the parameter's own range.
    pub value: f64,
    /// What the body actually measures there, in centimetres.
    pub achieved_cm: f64,
    /// Whether `value` sits at the parameter's own bound and still misses
    /// the target by more than [`TOLERANCE_CM`] — the model cannot reach
    /// this target, not a solver that gave up early.
    pub saturated: bool,
}

/// Runs a fixed-iteration secant search for the `t` (in `[lo, hi]`) that
/// makes `eval(t)` equal `target_cm`, starting from `t = 0` and a second
/// guess toward `target_cm` from there.
///
/// # Panics
/// Never: a degenerate step (equal function values at the two most recent
/// points) leaves `t` unchanged for that iteration rather than dividing by
/// zero, so every one of the fixed [`ITERATIONS`] steps always runs.
fn secant(mut eval: impl FnMut(f64) -> f64, target_cm: f64, lo: f64, hi: f64) -> Solved {
    let mut t0 = 0.0;
    let mut f0 = eval(t0) - target_cm;
    let mut t1 = if f0 <= 0.0 { 0.5 * hi } else { 0.5 * lo };
    let mut f1 = eval(t1) - target_cm;

    for _ in 0..ITERATIONS {
        let denom = f1 - f0;
        if denom.abs() > 1.0e-12 {
            let next = (t1 - f1 * (t1 - t0) / denom).clamp(lo, hi);
            t0 = t1;
            f0 = f1;
            t1 = next;
            f1 = eval(t1) - target_cm;
        }
    }

    let at_bound = (t1 - lo).abs() < 1.0e-9 || (t1 - hi).abs() < 1.0e-9;
    Solved {
        value: t1,
        achieved_cm: f1 + target_cm,
        saturated: at_bound && f1.abs() > TOLERANCE_CM,
    }
}

/// Solves one lever, or one tied pair of levers driven by the same value,
/// so that `extract` (read off a fresh [`Measures`] every trial) reports
/// `target_cm`.
///
/// `largo_lateral` and `brazo` each tie two levers this way; every other
/// lever and the phenotype are held fixed.
///
/// Cheap by construction, not by micro-optimizing the ring read: the
/// baseline (every row except `lever_ids`' own) is accumulated once, and
/// each of the fixed [`ITERATIONS`] trials only resets and re-applies the
/// handful of vertices those specific rows touch before re-measuring —
/// never the full 13,380-vertex, 26,756-triangle mesh, and never normals.
pub fn solve_girth(
    phenotype: &Phenotype,
    levers: &[f64; 20],
    lever_ids: &[u8],
    target_cm: f64,
    extract: impl Fn(&Measures) -> f32,
) -> Solved {
    let baked = decoded();
    let phens = phenotype::phens26(phenotype);
    let is_target_row = |lever: u8| lever_ids.contains(&lever);

    let mut baseline = template_positions_f64(baked);
    accumulate_except(
        baked,
        &phens,
        levers,
        &mut baseline,
        |row| matches!(row.kind, RowKind::Lever { lever, .. } if is_target_row(lever)),
    );

    let touched: Vec<usize> = baked
        .rows
        .iter()
        .filter(|row| matches!(row.kind, RowKind::Lever { lever, .. } if is_target_row(lever)))
        .flat_map(|row| {
            let start = row.offset as usize;
            let end = start + row.length as usize;
            baked.deltas[start..end].iter().map(|d| d.vertex as usize)
        })
        .collect();

    let mut working: Vec<f32> = baseline.iter().map(|&d| d as f32).collect();
    let eval = |t: f64| -> f64 {
        for &v in &touched {
            working[v * 3] = baseline[v * 3] as f32;
            working[v * 3 + 1] = baseline[v * 3 + 1] as f32;
            working[v * 3 + 2] = baseline[v * 3 + 2] as f32;
        }
        for row in &baked.rows {
            let RowKind::Lever { lever, incr } = row.kind else {
                continue;
            };
            if !is_target_row(lever) {
                continue;
            }
            let weight = lever_weight(t, incr);
            if weight == 0.0 {
                continue;
            }
            let start = row.offset as usize;
            let end = start + row.length as usize;
            for delta in &baked.deltas[start..end] {
                let base = delta.vertex as usize * 3;
                working[base] += (weight * f64::from(delta.dx) / 10_000.0) as f32;
                working[base + 1] += (weight * f64::from(delta.dy) / 10_000.0) as f32;
                working[base + 2] += (weight * f64::from(delta.dz) / 10_000.0) as f32;
            }
        }
        f64::from(extract(&measure::measure(&working)))
    };
    secant(eval, target_cm, -1.0, 1.0)
}

/// Solves `estatura` by varying the `height` phenotype input (not a row
/// lever — see [`Phenotype::height`]'s doc), so that the resulting mesh's
/// own bounding-box height reports `target_cm`.
///
/// Unlike [`solve_girth`], the rows that move are chosen by which of the
/// 26 phenotype keys a weighted row's mask names, not by a lever id — see
/// [`phenotype::height_key_mask`].
pub fn solve_height(phenotype: &Phenotype, levers: &[f64; 20], target_cm: f64) -> Solved {
    let baked = decoded();
    let height_mask = phenotype::height_key_mask();
    let is_height_row = |mask: u32| mask & height_mask != 0;

    let base_phens = phenotype::phens26(phenotype);
    let mut baseline = template_positions_f64(baked);
    accumulate_except(
        baked,
        &base_phens,
        levers,
        &mut baseline,
        |row| matches!(row.kind, RowKind::Weighted { mask } if is_height_row(mask)),
    );

    let touched: Vec<usize> = baked
        .rows
        .iter()
        .filter(|row| matches!(row.kind, RowKind::Weighted { mask } if is_height_row(mask)))
        .flat_map(|row| {
            let start = row.offset as usize;
            let end = start + row.length as usize;
            baked.deltas[start..end].iter().map(|d| d.vertex as usize)
        })
        .collect();

    let mut working: Vec<f32> = baseline.iter().map(|&d| d as f32).collect();
    let eval = |trial_height: f64| -> f64 {
        for &v in &touched {
            working[v * 3] = baseline[v * 3] as f32;
            working[v * 3 + 1] = baseline[v * 3 + 1] as f32;
            working[v * 3 + 2] = baseline[v * 3 + 2] as f32;
        }
        let mut trial = *phenotype;
        trial.height = trial_height;
        let phens = phenotype::phens26(&trial);
        for row in &baked.rows {
            let RowKind::Weighted { mask } = row.kind else {
                continue;
            };
            if !is_height_row(mask) {
                continue;
            }
            let weight = phenotype::row_weight(mask, &phens);
            if weight == 0.0 {
                continue;
            }
            let start = row.offset as usize;
            let end = start + row.length as usize;
            for delta in &baked.deltas[start..end] {
                let base = delta.vertex as usize * 3;
                working[base] += (weight * f64::from(delta.dx) / 10_000.0) as f32;
                working[base + 1] += (weight * f64::from(delta.dy) / 10_000.0) as f32;
                working[base + 2] += (weight * f64::from(delta.dz) / 10_000.0) as f32;
            }
        }
        f64::from(measure::measure(&working).height)
    };
    secant(eval, target_cm, 0.0, 1.0)
}
