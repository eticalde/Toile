use rayon::prelude::*;
use wide::{CmpGt, f32x8};

use super::bare::{Bare, Dropped};
use super::color::ColoredConstraints;
use super::layers::Layers;
use super::parallel::{self, MIN_CHUNK};
use super::ptr::{Buffers, Ptr};
use super::solver::GRAVITY;
use super::stage::Stage;
use super::state::{Seams, State};

/// [`super::substep_colored`] with the constraint arithmetic in batches of
/// eight.
///
/// Gathers and scatters stay scalar — neither NEON nor AVX2 has hardware
/// gather for this pattern — while the maths is vectorised. Each lane executes
/// the same IEEE operations in the same order as [`super::substep_colored`],
/// so the two are bit-identical, and a test gates that on a sheet whose
/// colours are wide enough to fill the batches.
///
/// It runs the same passes as that path and no more, and turns away the same
/// scenes on the same line, for the same reason.
///
/// # Errors
/// [`Dropped`] names the first pass this path would skip, and nothing moves.
pub fn substep_colored_simd(
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
    parallel::collide(b, scene.sdf, n);
    parallel::derive_velocities(b, n, dt);
    Ok(())
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
        unsafe { parallel::step_one(b, im, i, dt) };
    }
}

fn solve_colors(b: Buffers, inv_mass: &[f32], cc: &ColoredConstraints, scalar_inv_dt2: f32) {
    let inv_dt2 = f32x8::splat(scalar_inv_dt2);
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
        // The constraints of the colour that do not fill a batch of eight, in
        // the formulation the wide path is a batching of, because it is the
        // same function the scalar coloured path calls.
        for c in r.start + batches * 8..r.end {
            // SAFETY: the tail runs single-threaded after its colour's batches.
            unsafe { parallel::project(b, inv_mass, cons, c, scalar_inv_dt2) };
        }
    }
}
