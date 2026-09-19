use super::contact::{self, Floor};
use super::layers::Layers;
use super::sdf::SdfGrid;
use super::state::{DistanceConstraints, Seams, State};

#[cfg(test)]
mod tests;

// The scalar path is the reference formulation: the goldens are defined by
// it, and every other path must reproduce its bits.
/// What pulls on a particle, in metres per second squared.
pub const GRAVITY: f32 = -9.81;
pub(super) const DAMPING: f32 = 0.999;

/// Everything a substep asks of the world outside the cloth: the body it
/// falls on, the ground it may not fall through, and the pull on it.
///
/// The three travel together because a substep reads them together, and
/// because the alternative is an eighth parameter. Gravity is here rather
/// than a constant so that a garment can be sewn shut before it is allowed to
/// fall: [`Stage::weightless`] is that moment, and nothing else changes.
#[derive(Debug, Clone, Copy)]
#[must_use]
pub struct Stage<'a> {
    /// The body the cloth collides with.
    pub sdf: &'a SdfGrid,
    /// The plane under it, when the scene has one.
    pub floor: Floor,
    /// The pull on every particle, in metres per second squared.
    pub gravity: f32,
}

impl<'a> Stage<'a> {
    /// A body hanging in the void, under ordinary gravity: the scene every
    /// drape golden is taken in.
    pub const fn around(sdf: &'a SdfGrid) -> Stage<'a> {
        Stage {
            sdf,
            floor: Floor::none(),
            gravity: GRAVITY,
        }
    }

    /// The same, with ground under it.
    pub const fn on(self, floor: Floor) -> Stage<'a> {
        Stage {
            sdf: self.sdf,
            floor,
            gravity: self.gravity,
        }
    }

    /// The same, with nothing pulling.
    pub const fn weightless(self) -> Stage<'a> {
        Stage {
            sdf: self.sdf,
            floor: self.floor,
            gravity: 0.0,
        }
    }
}

/// One full XPBD substep.
///
/// Small steps: N substeps of one constraint iteration beat one step of N
/// iterations, so lambda starts from zero each substep and never accumulates.
///
/// A stage whose `floor` is [`Floor::none`] and whose `layers` is `None` runs
/// exactly the passes it has always run, in the order it has always run them.
pub fn substep(
    state: &mut State,
    cons: &DistanceConstraints,
    seams: &Seams,
    stage: &Stage,
    layers: Option<&mut Layers>,
    dt: f32,
) {
    let inv_dt2 = 1.0 / (dt * dt);
    integrate(state, stage.gravity, dt);
    solve_distance(state, cons, inv_dt2);
    if cons.strain_limit > 0.0 {
        limit_strain(state, cons);
    }
    if !seams.is_empty() {
        solve_seams(state, seams, inv_dt2);
    }
    // Ahead of the body and ahead of the ground, because those two get the
    // last word. Out past the band a baked field has no gradient left to carry
    // cloth back with, so a push that parts two layers must never be what puts
    // a particle inside a person; and nothing at all may end under the floor.
    // Cloth against cloth is the one contact here another pass can still
    // correct, so it is the one that goes first. It also wants the positions
    // the stretch and the seams have already had their say over: a fold the
    // constraints were about to pull out is not a fold.
    if let Some(layers) = layers {
        layers.separate(state);
    }
    collide(state, stage.sdf);
    // After the body and not before it. The ground is the one surface nothing
    // may end up under, and the field's gradient under a sole points down: a
    // particle the body pushes through the floor has to meet the plane after
    // that push, not before it. Still ahead of `derive_velocities`, so cloth
    // that lands is stopped the way the body stops it — moving `p` and
    // leaving `q` behind is what makes a contact a contact here.
    if let Some(y) = stage.floor.level() {
        rest_on_floor(state, y);
    }
    derive_velocities(state, dt);
}

/// Semi-implicit integration, saving the previous position for the velocity
/// derivation at the end of the substep.
fn integrate(state: &mut State, gravity: f32, dt: f32) {
    for i in 0..state.len() {
        state.qx[i] = state.px[i];
        state.qy[i] = state.py[i];
        state.qz[i] = state.pz[i];
        if state.inv_mass[i] > 0.0 {
            state.vy[i] += gravity * dt;
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

/// Rests the whole cloth on the ground plane.
///
/// A scene with no floor never reaches this, which is why the floor costs a
/// scene without one nothing at all — not a branch per particle, and not a
/// push multiplied by zero.
fn rest_on_floor(state: &mut State, y: f32) {
    for i in 0..state.len() {
        let (p, q) = contact::rest_on(
            y,
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
