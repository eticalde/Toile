use rayon::prelude::*;

use super::color::ColoredConstraints;
use super::contact;
use super::ptr::Buffers;
use super::sdf::SdfGrid;
use super::solver::{DAMPING, GRAVITY};
use super::state::State;

/// Below this many items a phase is not worth splitting across threads.
pub(super) const MIN_CHUNK: usize = 4096;

/// One XPBD substep over colour-partitioned constraints.
///
/// Produces the same bits on one thread or eight: within a colour the writes
/// are disjoint and there are no reductions, so no result depends on the
/// scheduler.
///
/// Unlike [`super::substep`], this path carries neither seams nor strain
/// limiting.
pub fn substep_colored(state: &mut State, cc: &ColoredConstraints, sdf: &SdfGrid, dt: f32) {
    let n = state.len();
    let b = Buffers::of(state);
    let inv_mass = &state.inv_mass;

    integrate(b, inv_mass, n, dt);
    solve_colors(b, inv_mass, cc, 1.0 / (dt * dt));
    collide(b, sdf, n);
    derive_velocities(b, n, dt);
}

fn integrate(b: Buffers, inv_mass: &[f32], n: usize, dt: f32) {
    (0..n)
        .into_par_iter()
        .with_min_len(MIN_CHUNK)
        .for_each(|i| {
            // SAFETY: every index is written by exactly one iteration.
            unsafe {
                *b.qx.at(i) = *b.px.at(i);
                *b.qy.at(i) = *b.py.at(i);
                *b.qz.at(i) = *b.pz.at(i);
                if inv_mass[i] > 0.0 {
                    *b.vy.at(i) += GRAVITY * dt;
                    *b.px.at(i) += *b.vx.at(i) * dt;
                    *b.py.at(i) += *b.vy.at(i) * dt;
                    *b.pz.at(i) += *b.vz.at(i) * dt;
                }
            }
        });
}

fn solve_colors(b: Buffers, inv_mass: &[f32], cc: &ColoredConstraints, inv_dt2: f32) {
    let cons = &cc.cons;
    for r in &cc.ranges {
        (r.start..r.end)
            .into_par_iter()
            .with_min_len(MIN_CHUNK)
            .for_each(|c| {
                let (ia, ib) = (cons.a[c] as usize, cons.b[c] as usize);
                let (wa, wb) = (inv_mass[ia], inv_mass[ib]);
                let w = wa + wb;
                if w == 0.0 {
                    return;
                }
                // SAFETY: no two constraints in a colour share a vertex, so
                // the reads and writes of this phase are disjoint across
                // iterations.
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
            });
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
                let (p, q) = contact::resolve(sdf, eps, p, q);
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
