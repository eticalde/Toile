use rayon::prelude::*;

use super::bare::{Bare, Dropped};
use super::color::ColoredConstraints;
use super::contact::{self, Grip};
use super::layers::Layers;
use super::ptr::Buffers;
use super::sdf::SdfGrid;
use super::solver::{DAMPING, GRAVITY};
use super::stage::Stage;
use super::state::{DistanceConstraints, Seams, State};

/// Below this many items a phase is not worth splitting across threads.
pub(super) const MIN_CHUNK: usize = 4096;

/// One XPBD substep over colour-partitioned constraints.
///
/// Takes what [`super::substep`] takes, and refuses the scenes it would only
/// half solve: it runs the stretch constraints and the body, and none of the
/// other passes the reference path has. The refusal is made here, on the very
/// values this call will integrate, because a proof earned once outside the
/// loop answers for the scene as it stood then.
///
/// Produces the same bits on one thread or eight: within a colour the writes
/// are disjoint and there are no reductions, so no result depends on the
/// scheduler. Not the same bits as [`super::substep`], which sweeps the
/// constraints in their stored order while this sweeps a permuted set colour
/// by colour: the two orders are different arithmetic.
///
/// # Errors
/// [`Dropped`] names the first pass this path would skip, and nothing moves.
pub fn substep_colored(
    state: &mut State,
    cons: &ColoredConstraints,
    seams: &Seams,
    stage: &Stage<'_>,
    layers: Option<&Layers>,
    dt: f32,
) -> Result<(), Dropped> {
    let scene = Bare::of(cons, seams, stage, layers)?;
    let n = state.len();
    let b = Buffers::of(state);
    let inv_mass = &state.inv_mass;

    integrate(b, inv_mass, n, dt);
    solve_colors(b, inv_mass, scene.cons, 1.0 / (dt * dt));
    collide(b, scene.sdf, n);
    derive_velocities(b, n, dt);
    Ok(())
}

fn integrate(b: Buffers, inv_mass: &[f32], n: usize, dt: f32) {
    (0..n)
        .into_par_iter()
        .with_min_len(MIN_CHUNK)
        .for_each(|i| {
            // SAFETY: every index is written by exactly one iteration.
            unsafe { step_one(b, inv_mass[i], i, dt) };
        });
}

/// One particle carried forward a step, saving where it came from.
///
/// Written once for the same reason [`project`] is: the wide path's remainder
/// integrates the last few particles scalar, and a remainder that integrates
/// them differently from the batches is a seam in the middle of the cloth.
///
/// # Safety
/// Nothing else may be reading or writing particle `i` for the duration of the
/// call.
#[inline]
pub(super) unsafe fn step_one(b: Buffers, inv_mass: f32, i: usize, dt: f32) {
    // SAFETY: the caller guarantees the particle is its own while this runs.
    unsafe {
        *b.qx.at(i) = *b.px.at(i);
        *b.qy.at(i) = *b.py.at(i);
        *b.qz.at(i) = *b.pz.at(i);
        if inv_mass > 0.0 {
            *b.vy.at(i) += GRAVITY * dt;
            *b.px.at(i) += *b.vx.at(i) * dt;
            *b.py.at(i) += *b.vy.at(i) * dt;
            *b.pz.at(i) += *b.vz.at(i) * dt;
        }
    }
}

fn solve_colors(b: Buffers, inv_mass: &[f32], cc: &ColoredConstraints, inv_dt2: f32) {
    let cons = &cc.cons;
    for r in &cc.ranges {
        (r.start..r.end)
            .into_par_iter()
            .with_min_len(MIN_CHUNK)
            .for_each(|c| {
                // SAFETY: no two constraints in a colour share a vertex, so
                // the reads and writes of this phase are disjoint across
                // iterations.
                unsafe { project(b, inv_mass, cons, c, inv_dt2) };
            });
    }
}

/// One distance constraint, projected onto its pair through the raw buffers.
///
/// The arithmetic of [`super::solver`]'s `solve_one`, written once for the two
/// coloured paths instead of twice: the rayon phase above and the batch
/// remainder [`super::substep_colored_simd`] leaves behind both come here, so
/// an edit to the formulation cannot reach one and miss the other.
///
/// # Safety
/// Nothing else may be reading or writing the two vertices `c` names for the
/// duration of the call.
#[inline]
pub(super) unsafe fn project(
    b: Buffers,
    inv_mass: &[f32],
    cons: &DistanceConstraints,
    c: usize,
    inv_dt2: f32,
) {
    let (ia, ib) = (cons.a[c] as usize, cons.b[c] as usize);
    let (wa, wb) = (inv_mass[ia], inv_mass[ib]);
    let w = wa + wb;
    if w == 0.0 {
        return;
    }
    // SAFETY: the caller guarantees the pair is its own while this runs.
    unsafe {
        let dx = *b.px.at(ib) - *b.px.at(ia);
        let dy = *b.py.at(ib) - *b.py.at(ia);
        let dz = *b.pz.at(ib) - *b.pz.at(ia);
        let len = (dx * dx + dy * dy + dz * dz).sqrt();
        if len <= 1.0e-9 {
            return;
        }
        let alpha = cons.compliance[c] * inv_dt2;
        let corr = (len - cons.rest[c]) / ((w + alpha) * len);
        let (sx, sy, sz) = (corr * dx, corr * dy, corr * dz);
        *b.px.at(ia) += wa * sx;
        *b.py.at(ia) += wa * sy;
        *b.pz.at(ia) += wa * sz;
        *b.px.at(ib) -= wb * sx;
        *b.py.at(ib) -= wb * sy;
        *b.pz.at(ib) -= wb * sz;
    }
}

pub(super) fn collide(b: Buffers, sdf: &SdfGrid, n: usize) {
    let eps = sdf.cell * 0.5;
    (0..n)
        .into_par_iter()
        .with_min_len(MIN_CHUNK)
        .for_each(|i| {
            // SAFETY: one index per iteration, disjoint.
            unsafe {
                let (p, q) = b.pq(i);
                let (p, q) = contact::resolve(sdf, eps, Grip::slipping(), p, q);
                b.set_pq(i, p, q);
            }
        });
}

pub(super) fn derive_velocities(b: Buffers, n: usize, dt: f32) {
    let inv_dt = 1.0 / dt;
    (0..n)
        .into_par_iter()
        .with_min_len(MIN_CHUNK)
        .for_each(|i| {
            // SAFETY: one index per iteration, disjoint.
            unsafe {
                *b.vx.at(i) = (*b.px.at(i) - *b.qx.at(i)) * inv_dt * DAMPING;
                *b.vy.at(i) = (*b.py.at(i) - *b.qy.at(i)) * inv_dt * DAMPING;
                *b.vz.at(i) = (*b.pz.at(i) - *b.qz.at(i)) * inv_dt * DAMPING;
            }
        });
}
