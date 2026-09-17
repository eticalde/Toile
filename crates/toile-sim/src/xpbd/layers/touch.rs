use crate::xpbd::state::State;
use crate::xpbd::vector::{dot, sub};

/// How much of a parting correction the previous position carries with it.
///
/// All of it and the layers are moved without being stirred, which is what
/// [`crate::xpbd::lift_out_of`] does — but then nothing ever takes the
/// approach out of the velocity, and two layers pressed together by gravity
/// go on gathering speed against a contact that never slows them until they
/// cross the whole thickness inside one substep. Measured on a folded sheet,
/// that is about a hundred and fifty substeps: a quarter of a second.
///
/// None of it and every contact is a bounce.
///
/// Half leaves the closing speed at a fixed point of two gravities a substep,
/// 33 mm/s, which is a sink of five hundredths of a millimetre. It is also
/// the bargain the field's own contact and the floor already strike, where
/// half of what a contact corrects is left behind as motion.
const FOLLOW: f32 = 0.5;

/// Parts one vertex from one triangle, and says whether it had to.
///
/// PBD reads velocity as the difference between the two positions, so the
/// part of the correction `q` does not follow is what reaches the velocity
/// the substep is about to derive. Re-seating cloth once and holding two
/// layers apart for a whole drape are different jobs: see [`FOLLOW`].
pub(super) fn part(state: &mut State, v: usize, t: [usize; 3], thickness: f32) -> bool {
    let p = at(state, v);
    let corner = [at(state, t[0]), at(state, t[1]), at(state, t[2])];
    let bary = closest_on(p, corner);
    let near = [
        bary[0] * corner[0][0] + bary[1] * corner[1][0] + bary[2] * corner[2][0],
        bary[0] * corner[0][1] + bary[1] * corner[1][1] + bary[2] * corner[2][1],
        bary[0] * corner[0][2] + bary[1] * corner[1][2] + bary[2] * corner[2][2],
    ];
    let gap = sub(p, near);
    let square = dot(gap, gap);
    // Exactly on the triangle there is no side to be parted toward, the same
    // way the saturated interior of a baked field has no normal to push
    // along. Left where it is, for the stretch constraints to draw out.
    if square >= thickness * thickness || square <= 0.0 {
        return false;
    }

    let dist = square.sqrt();
    let out = [gap[0] / dist, gap[1] / dist, gap[2] / dist];
    let wv = state.inv_mass[v];
    let wt = [
        state.inv_mass[t[0]],
        state.inv_mass[t[1]],
        state.inv_mass[t[2]],
    ];
    // The triangle resists at the contact point rather than at its corners, so
    // each corner counts for the square of the share of the correction it
    // carries. Three pinned corners and a pinned vertex leave nothing to move.
    let share =
        wv + wt[0] * bary[0] * bary[0] + wt[1] * bary[1] * bary[1] + wt[2] * bary[2] * bary[2];
    if share <= 0.0 {
        return false;
    }
    let lambda = (thickness - dist) / share;

    shift(state, v, scaled(out, lambda * wv));
    for c in 0..3 {
        shift(state, t[c], scaled(out, -lambda * wt[c] * bary[c]));
    }
    true
}

/// Where a point lies closest on a triangle, as one weight per corner.
///
/// Ericson's walk over the triangle's seven Voronoi regions: three corners,
/// three edges, and the face. It costs `+ - * /` and nothing else, which is
/// what lets a contact of this shape into the geometry at all.
fn closest_on(p: [f32; 3], t: [[f32; 3]; 3]) -> [f32; 3] {
    let ab = sub(t[1], t[0]);
    let ac = sub(t[2], t[0]);
    let ap = sub(p, t[0]);
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return [1.0, 0.0, 0.0];
    }

    let bp = sub(p, t[1]);
    let (d3, d4) = (dot(ab, bp), dot(ac, bp));
    if d3 >= 0.0 && d4 <= d3 {
        return [0.0, 1.0, 0.0];
    }
    let edge_ab = d1 * d4 - d3 * d2;
    if edge_ab <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = ratio(d1, d1 - d3);
        return [1.0 - v, v, 0.0];
    }

    let cp = sub(p, t[2]);
    let (d5, d6) = (dot(ab, cp), dot(ac, cp));
    if d6 >= 0.0 && d5 <= d6 {
        return [0.0, 0.0, 1.0];
    }
    let edge_ac = d5 * d2 - d1 * d6;
    if edge_ac <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = ratio(d2, d2 - d6);
        return [1.0 - w, 0.0, w];
    }

    let edge_bc = d3 * d6 - d5 * d4;
    if edge_bc <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        let w = ratio(d4 - d3, (d4 - d3) + (d5 - d6));
        return [0.0, 1.0 - w, w];
    }

    let sum = edge_ab + edge_ac + edge_bc;
    if sum <= 0.0 {
        return [1.0, 0.0, 0.0];
    }
    let (v, w) = (edge_ac / sum, edge_ab / sum);
    [1.0 - v - w, v, w]
}

/// A ratio whose denominator a triangle with no area drives to zero.
fn ratio(num: f32, den: f32) -> f32 {
    if den > 0.0 { num / den } else { 0.0 }
}

fn at(state: &State, i: usize) -> [f32; 3] {
    [state.px[i], state.py[i], state.pz[i]]
}

fn scaled(v: [f32; 3], by: f32) -> [f32; 3] {
    [v[0] * by, v[1] * by, v[2] * by]
}

fn shift(state: &mut State, i: usize, by: [f32; 3]) {
    state.px[i] += by[0];
    state.py[i] += by[1];
    state.pz[i] += by[2];
    state.qx[i] += FOLLOW * by[0];
    state.qy[i] += FOLLOW * by[1];
    state.qz[i] += FOLLOW * by[2];
}
