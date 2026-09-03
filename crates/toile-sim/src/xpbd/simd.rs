use rayon::prelude::*;
use wide::{CmpGt, f32x8};

use super::color::ColoredConstraints;
use super::parallel::{self, MIN_CHUNK};
use super::ptr::{Buffers, Ptr};
use super::sdf::SdfGrid;
use super::solver::GRAVITY;
use super::state::State;

/// [`super::substep_colored`] with the constraint arithmetic in batches of
/// eight.
///
/// Gathers and scatters stay scalar — neither NEON nor AVX2 has hardware
/// gather for this pattern — while the maths is vectorised. Each lane executes
/// the same IEEE operations as the scalar path, so the result is bit-identical
/// to it.
///
/// Like [`super::substep_colored`], this path carries neither seams nor strain
/// limiting.
pub fn substep_colored_simd(state: &mut State, cc: &ColoredConstraints, sdf: &SdfGrid, dt: f32) {
    let n = state.len();
    let b = Buffers::of(state);
    let inv_mass = &state.inv_mass;

    integrate(b, inv_mass, n, dt);
    solve_colors(b, inv_mass, cc, dt);
    parallel::collide(b, sdf, n);
    parallel::derive_velocities(b, n, dt);
}

/// Positions are contiguous, so integration vectorises directly over blocks of
/// eight; the remainder is handled scalar.
fn integrate(b: Buffers, inv_mass: &[f32], n: usize, dt: f32) {
    let blocks = n / 8;
    (0..blocks)
        .into_par_iter()
        .with_min_len(MIN_CHUNK / 8)
        .for_each(|blk| {
            let i = blk * 8;
            // SAFETY: blocks are disjoint, one per iteration.
            unsafe {
                let load = |p: Ptr| f32x8::from(std::array::from_fn(|l| *p.at(i + l)));
                let store = |p: Ptr, v: f32x8| {
                    for (l, x) in v.to_array().iter().enumerate() {
                        *p.at(i + l) = *x;
                    }
                };
                let (pxv, pyv, pzv) = (load(b.px), load(b.py), load(b.pz));
                store(b.qx, pxv);
                store(b.qy, pyv);
                store(b.qz, pzv);
                let w = f32x8::from(std::array::from_fn(|l| inv_mass[i + l]));
                let active = w.cmp_gt(f32x8::splat(0.0));
                let vyv = load(b.vy) + active.blend(f32x8::splat(GRAVITY * dt), f32x8::splat(0.0));
                store(b.vy, vyv);
                let dtv = f32x8::splat(dt);
                store(
                    b.px,
                    pxv + active.blend(load(b.vx) * dtv, f32x8::splat(0.0)),
                );
                store(b.py, pyv + active.blend(vyv * dtv, f32x8::splat(0.0)));
                store(
                    b.pz,
                    pzv + active.blend(load(b.vz) * dtv, f32x8::splat(0.0)),
                );
            }
        });
    for (i, &im) in inv_mass.iter().enumerate().skip(blocks * 8) {
        // SAFETY: the tail is disjoint from every block and runs
        // single-threaded.
        unsafe {
            *b.qx.at(i) = *b.px.at(i);
            *b.qy.at(i) = *b.py.at(i);
            *b.qz.at(i) = *b.pz.at(i);
            if im > 0.0 {
                *b.vy.at(i) += GRAVITY * dt;
                *b.px.at(i) += *b.vx.at(i) * dt;
                *b.py.at(i) += *b.vy.at(i) * dt;
                *b.pz.at(i) += *b.vz.at(i) * dt;
            }
        }
    }
}

fn solve_colors(b: Buffers, inv_mass: &[f32], cc: &ColoredConstraints, dt: f32) {
    let inv_dt2 = f32x8::splat(1.0 / (dt * dt));
    let cons = &cc.cons;
    for r in &cc.ranges {
        let batches = (r.end - r.start) / 8;
        (0..batches)
            .into_par_iter()
            .with_min_len(MIN_CHUNK / 8)
            .for_each(|bt| {
                let c0 = r.start + bt * 8;
                let ia: [usize; 8] = std::array::from_fn(|l| cons.a[c0 + l] as usize);
                let ib: [usize; 8] = std::array::from_fn(|l| cons.b[c0 + l] as usize);
                // SAFETY: no two constraints in a colour share a vertex, so
                // the eight gathers of a batch read positions no other batch
                // of the colour writes, and the scatters are disjoint.
                unsafe {
                    let gather = |p: Ptr, idx: &[usize; 8]| {
                        f32x8::from(std::array::from_fn(|l| *p.at(idx[l])))
                    };
                    let wa = f32x8::from(std::array::from_fn(|l| inv_mass[ia[l]]));
                    let wb = f32x8::from(std::array::from_fn(|l| inv_mass[ib[l]]));
                    let w = wa + wb;
                    let dx = gather(b.px, &ib) - gather(b.px, &ia);
                    let dy = gather(b.py, &ib) - gather(b.py, &ia);
                    let dz = gather(b.pz, &ib) - gather(b.pz, &ia);
                    let len = (dx * dx + dy * dy + dz * dz).sqrt();
                    let rest = f32x8::from(std::array::from_fn(|l| cons.rest[c0 + l]));
                    let compliance = f32x8::from(std::array::from_fn(|l| cons.compliance[c0 + l]));
                    let alpha = compliance * inv_dt2;
                    let valid = w.cmp_gt(f32x8::splat(0.0)) & len.cmp_gt(f32x8::splat(1.0e-9));
                    let corr = valid.blend((len - rest) / ((w + alpha) * len), f32x8::splat(0.0));
                    let sx = (corr * dx).to_array();
                    let sy = (corr * dy).to_array();
                    let sz = (corr * dz).to_array();
                    let waa = wa.to_array();
                    let wba = wb.to_array();
                    for l in 0..8 {
                        *b.px.at(ia[l]) += waa[l] * sx[l];
                        *b.py.at(ia[l]) += waa[l] * sy[l];
                        *b.pz.at(ia[l]) += waa[l] * sz[l];
                        *b.px.at(ib[l]) -= wba[l] * sx[l];
                        *b.py.at(ib[l]) -= wba[l] * sy[l];
                        *b.pz.at(ib[l]) -= wba[l] * sz[l];
                    }
                }
            });
        solve_color_tail(b, inv_mass, cons, r.start + batches * 8..r.end, dt);
    }
}

/// The constraints of a colour that do not fill a batch of eight, in the same
/// formulation as the scalar path.
fn solve_color_tail(
    b: Buffers,
    inv_mass: &[f32],
    cons: &super::state::DistanceConstraints,
    range: std::ops::Range<usize>,
    dt: f32,
) {
    for c in range {
        let (ia, ib) = (cons.a[c] as usize, cons.b[c] as usize);
        let (wa, wb) = (inv_mass[ia], inv_mass[ib]);
        let w = wa + wb;
        if w == 0.0 {
            continue;
        }
        // SAFETY: the tail runs single-threaded after its colour's batches.
        unsafe {
            let dx = *b.px.at(ib) - *b.px.at(ia);
            let dy = *b.py.at(ib) - *b.py.at(ia);
            let dz = *b.pz.at(ib) - *b.pz.at(ia);
            let len = (dx * dx + dy * dy + dz * dz).sqrt();
            if len <= 1.0e-9 {
                continue;
            }
            let alpha = cons.compliance[c] * (1.0 / (dt * dt));
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
}
