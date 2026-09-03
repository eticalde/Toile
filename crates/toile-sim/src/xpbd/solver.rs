use super::contact;
use super::sdf::SdfGrid;
use super::state::{DistanceConstraints, Seams, State};

/// The scalar path is the reference formulation: the goldens are defined by
/// it, and every other path must reproduce its bits.
pub(super) const GRAVITY: f32 = -9.81;
pub(super) const DAMPING: f32 = 0.999;

/// One full XPBD substep.
///
/// Small steps: N substeps of one constraint iteration beat one step of N
/// iterations, so lambda starts from zero each substep and never accumulates.
pub fn substep(
    state: &mut State,
    cons: &DistanceConstraints,
    seams: &Seams,
    sdf: &SdfGrid,
    dt: f32,
) {
    let inv_dt2 = 1.0 / (dt * dt);
    integrate(state, dt);
    solve_distance(state, cons, inv_dt2);
    if cons.strain_limit > 0.0 {
        limit_strain(state, cons);
    }
    if !seams.is_empty() {
        solve_seams(state, seams, inv_dt2);
    }
    collide(state, sdf);
    derive_velocities(state, dt);
}

/// Semi-implicit integration, saving the previous position for the velocity
/// derivation at the end of the substep.
fn integrate(state: &mut State, dt: f32) {
    for i in 0..state.len() {
        state.qx[i] = state.px[i];
        state.qy[i] = state.py[i];
        state.qz[i] = state.pz[i];
        if state.inv_mass[i] > 0.0 {
            state.vy[i] += GRAVITY * dt;
            state.px[i] += state.vx[i] * dt;
            state.py[i] += state.vy[i] * dt;
            state.pz[i] += state.vz[i] * dt;
        }
    }
}

/// Sequential Gauss-Seidel over the constraints, in their stored order.
fn solve_distance(state: &mut State, cons: &DistanceConstraints, inv_dt2: f32) {
    for c in 0..cons.len() {
        let (ia, ib) = (cons.a[c] as usize, cons.b[c] as usize);
        let (wa, wb) = (state.inv_mass[ia], state.inv_mass[ib]);
        let w = wa + wb;
        if w == 0.0 {
            continue;
        }
        let dx = state.px[ib] - state.px[ia];
        let dy = state.py[ib] - state.py[ia];
        let dz = state.pz[ib] - state.pz[ia];
        let len = (dx * dx + dy * dy + dz * dz).sqrt();
        if len <= 1.0e-9 {
            continue;
        }
        let alpha = cons.compliance[c] * inv_dt2;
        let corr = (len - cons.rest[c]) / ((w + alpha) * len);
        apply_pair(state, (ia, ib), (wa, wb), [corr * dx, corr * dy, corr * dz]);
    }
}

/// Hard post-solve clamp of over-elongated edges.
///
/// Sweeps alternate direction: a sequential clamp re-stretches the neighbours
/// ahead of it, and on long chains four same-direction passes never converge
/// while alternating ones do.
fn limit_strain(state: &mut State, cons: &DistanceConstraints) {
    let m = cons.len();
    for sweep in 0..cons.strain_sweeps.max(4) {
        for idx in 0..m {
            let c = if sweep % 2 == 0 { idx } else { m - 1 - idx };
            let (ia, ib) = (cons.a[c] as usize, cons.b[c] as usize);
            let (wa, wb) = (state.inv_mass[ia], state.inv_mass[ib]);
            let w = wa + wb;
            if w == 0.0 {
                continue;
            }
            let dx = state.px[ib] - state.px[ia];
            let dy = state.py[ib] - state.py[ia];
            let dz = state.pz[ib] - state.pz[ia];
            let len = (dx * dx + dy * dy + dz * dz).sqrt();
            let max_len = cons.rest[c] * cons.strain_limit;
            if len <= max_len {
                continue;
            }
            let corr = (len - max_len) / (w * len);
            apply_pair(state, (ia, ib), (wa, wb), [corr * dx, corr * dy, corr * dz]);
        }
    }
}

/// Seam attachments: rest length zero, correction capped per substep.
fn solve_seams(state: &mut State, seams: &Seams, inv_dt2: f32) {
    let alpha = seams.compliance * inv_dt2;
    for _ in 0..seams.iterations.max(1) {
        for k in 0..seams.len() {
            let (ia, ib) = (seams.a[k] as usize, seams.b[k] as usize);
            let (wa, wb) = (state.inv_mass[ia], state.inv_mass[ib]);
            let w = wa + wb;
            if w == 0.0 {
                continue;
            }
            let dx = state.px[ib] - state.px[ia];
            let dy = state.py[ib] - state.py[ia];
            let dz = state.pz[ib] - state.pz[ia];
            let len = (dx * dx + dy * dy + dz * dz).sqrt();
            if len <= 1.0e-9 {
                continue;
            }
            let corr = (len / (w + alpha)).min(seams.max_step);
            let scale = corr / len;
            apply_pair(
                state,
                (ia, ib),
                (wa, wb),
                [scale * dx, scale * dy, scale * dz],
            );
        }
    }
}

/// Scatters a symmetric positional correction onto a constrained pair.
#[inline]
fn apply_pair(state: &mut State, (ia, ib): (usize, usize), (wa, wb): (f32, f32), s: [f32; 3]) {
    state.px[ia] += wa * s[0];
    state.py[ia] += wa * s[1];
    state.pz[ia] += wa * s[2];
    state.px[ib] -= wb * s[0];
    state.py[ib] -= wb * s[1];
    state.pz[ib] -= wb * s[2];
}

fn collide(state: &mut State, sdf: &SdfGrid) {
    let eps = sdf.cell * 0.5;
    for i in 0..state.len() {
        let (p, q) = contact::resolve(
            sdf,
            eps,
            [state.px[i], state.py[i], state.pz[i]],
            [state.qx[i], state.qy[i], state.qz[i]],
        );
        [state.px[i], state.py[i], state.pz[i]] = p;
        [state.qx[i], state.qy[i], state.qz[i]] = q;
    }
}

fn derive_velocities(state: &mut State, dt: f32) {
    let inv_dt = 1.0 / dt;
    for i in 0..state.len() {
        state.vx[i] = (state.px[i] - state.qx[i]) * inv_dt * DAMPING;
        state.vy[i] = (state.py[i] - state.qy[i]) * inv_dt * DAMPING;
        state.vz[i] = (state.pz[i] - state.qz[i]) * inv_dt * DAMPING;
    }
}
