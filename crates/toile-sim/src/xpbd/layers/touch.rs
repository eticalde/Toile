use crate::xpbd::state::State;
use crate::xpbd::vector::{dot, sub};

/// Share of an overlap one contact takes out in one substep.
///
/// A vertex in a heap is in several contacts at once, with a layer on either
/// side of it and the stretch of its own cloth pulling across both. One sweep
/// that takes each of them out whole has every contact undo its neighbours,
/// which is Bridson's reason for a quarter, and it is what was measured on a
/// garment heaped on the ground: at a quarter the heap stops twitching in
/// about half the substeps it needs at a half, and corrected whole it had not
/// stopped when the run ended.
pub(super) const RELAXATION: f32 = 0.25;

/// Most speed one contact may hand its pair in one substep, in metres per
/// second.
///
/// An overlap taken out inside one substep parts the pair at that overlap
/// over the substep, and at six hundred substeps a second a millimetre is
/// more than half a metre a second handed to cloth that was lying still.
/// Capped, a deep overlap is walked out over several substeps instead, which
/// is Macklin's remedy for small steps. It bounds a contact and not a
/// particle: a vertex over several triangles is handed this by each of them.
///
/// It is a trade against separation. At the engine's own density this binds
/// on the deepest seventh of the thickness only, and the heap there kept a
/// third fewer crossings than with no cap at all. Far lower, cloth arriving
/// faster than the cap sinks through before it is stopped: at a tenth of a
/// metre a second a coarse heap keeps two to three times the crossings.
pub(super) const PARTING_SPEED: f32 = 0.25;

/// The most one contact may correct in a substep of `dt` seconds, in metres.
pub(super) fn reach(dt: f32) -> f32 {
    PARTING_SPEED * dt
}

/// Parts one vertex from one triangle, and says whether it had to.
///
/// Only the position moves. PBD reads velocity as what a substep left between
/// the previous position and this one, and most contacts in a heap are
/// standing ones: the cloth's own stretch pulls a vertex in, the contact puts
/// it back, and it ends the substep where it began. A previous position left
/// alone reads that as no motion, which is what it was. One that follows any
/// share of the correction ends the substep displaced from a vertex that
/// never moved, and that is speed out of nothing in every substep: measured,
/// a heap whose previous positions follow half the correction never comes
/// within three orders of the energy it sleeps at, and crosses itself half
/// as much again, because the speed a contact takes out of an approach is
/// the share it does not follow.
pub(super) fn part(state: &mut State, v: usize, t: [usize; 3], thickness: f32, reach: f32) -> bool {
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
    let lambda = (RELAXATION * (thickness - dist)).min(reach) / share;

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
}
