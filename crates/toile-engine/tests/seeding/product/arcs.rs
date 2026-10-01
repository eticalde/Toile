use std::f64::consts::TAU;

use toile_engine::couture::Layout;
use toile_engine::session::Session;

use super::outline::Outline;
use crate::fit::meshed;

/// How near two directions have to be to read as the one junction, in turns.
///
/// The mesh puts its own boundary vertices about a hundredth of a turn apart on
/// this block, and the two edges a junction butts together are placed from the
/// same walked cloth, so they agree to the last bits of the `f32` the release
/// is written in. A millionth of a turn is four orders under the one and three
/// over the other: nothing else can land in that gap.
const SAME: f64 = 1.0e-6;

/// How many places across a piece's cloth are read at each ordinate.
///
/// Enough that no step of the walk is anywhere near a half turn, so which way
/// round the surface it went is never in doubt.
const ACROSS: u32 = 64;

/// Every ordinate either outline turns a corner at, ascending and without
/// repeats.
///
/// The sample the arcs are read at, and not a round number of them. Between two
/// of these both outlines run straight, so the cloth walked to a point and the
/// hoop it is divided by are both linear in the ordinate and the turn between
/// them is monotone: whatever happens between two rungs has already happened at
/// one of them.
fn rungs(outlines: &[Outline]) -> Vec<f64> {
    let mut all: Vec<f64> = outlines.iter().flat_map(Outline::corners).collect();
    all.sort_by(f64::total_cmp);
    all.dedup();
    all
}

/// The sample directions collapsed into the places round the surface they
/// actually land on, each with the pieces that have cloth there.
///
/// Coincident samples have to become one place before the order can be read at
/// all: at a junction two pieces are at the same direction, and left as two
/// entries the sort may hand them back in either order — which reads as one
/// piece stepping over the other and back.
fn grouped(round: &mut [(f64, usize)]) -> Vec<(f64, u32)> {
    round.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut places: Vec<(f64, u32)> = Vec::new();
    for &(u, at) in round.iter() {
        match places.last_mut() {
            Some(last) if u - last.0 <= SAME => last.1 |= 1 << at,
            _ => places.push((u, 1 << at)),
        }
    }
    // The last place may be the first one come round again, a hair under a
    // whole turn from it.
    if places.len() > 1 {
        let (first, last) = (places[0], places[places.len() - 1]);
        if 1.0 - last.0 + first.0 <= SAME {
            places[0].1 |= last.1;
            places.pop();
        }
    }
    places
}

/// An angle brought into the half turn either side of zero.
fn short(d: f64) -> f64 {
    let wrapped = d.rem_euclid(TAU);
    if wrapped > TAU / 2.0 {
        wrapped - TAU
    } else {
        wrapped
    }
}

/// Where the pieces sit round the surface, and the distance that cannot say it.
///
/// The closest approach between two vertices is the wrong instrument, and it
/// reads like the right one. The pieces are rolled onto arcs that *abut*: the
/// front's centre front and the back's centre back are unsewn edges the walk
/// butts together to close the tube, so the smallest gap in the product is
/// there and is zero by construction — measured, 0.28 mm, the mesh's own vertex
/// spacing. A distance above zero is proof they are *not* inside each other.
///
/// What is read instead is the ordering. Going once round the surface at any
/// ordinate, the cloth may change piece only where two pieces meet: arcs that
/// abut give no change anywhere else, one piece reaching into the other gives a
/// change out in the open, and so does a hoop longer than the cloth on it.
///
/// # Panics
/// If the cloth changes piece away from a junction, or the arcs do not between
/// them cover the surface exactly once where every piece is there.
pub fn one_arc_each(session: &Session, ring: &Layout) {
    let outlines: Vec<Outline> = meshed(session).iter().map(Outline::of).collect();
    let ladder = rungs(&outlines);
    let mut open_change: Option<(f64, f64, u32, u32)> = None;
    let mut worst_cover = (0.0f64, 0.0f64);
    for &y in &ladder {
        let mut round: Vec<(f64, usize)> = Vec::new();
        let (mut swept, mut present) = (0.0f64, 0usize);
        for (at, outline) in outlines.iter().enumerate() {
            let Some((lo, hi)) = outline.scan(y) else {
                continue;
            };
            present += 1;
            let mut last: Option<f64> = None;
            for k in 0..=ACROSS {
                let x = lo + (hi - lo) * f64::from(k) / f64::from(ACROSS);
                let Some(q) = ring.point(at, [x, y]) else {
                    continue;
                };
                let turn = f64::from(q[2] - ring.axis[1]).atan2(f64::from(q[0] - ring.axis[0]));
                round.push((turn.rem_euclid(TAU) / TAU, at));
                if let Some(prev) = last {
                    swept += short(turn - prev).abs();
                }
                last = Some(turn);
            }
        }
        let groups = grouped(&mut round);
        for k in 0..groups.len() {
            let (here, next) = (groups[k], groups[(k + 1) % groups.len()]);
            if here.1 & next.1 == 0 && open_change.is_none() {
                open_change = Some((y, (next.0 - here.0).rem_euclid(1.0), here.1, next.1));
            }
        }
        if present == outlines.len() && (swept - TAU).abs() > worst_cover.0 {
            worst_cover = ((swept - TAU).abs(), y);
        }
    }
    println!(
        "release: over {} ordinates the cloth changes piece only where two pieces meet, and \
         where every piece is there the arcs cover the surface to within {:.2e} of a turn \
         (worst at {:.5})",
        ladder.len(),
        worst_cover.0 / TAU,
        worst_cover.1
    );
    assert!(
        open_change.is_none(),
        "the cloth changes piece away from any junction, so one piece is inside \
         the other or a stretch of surface carries nothing: {open_change:?} as \
         ordinate, turns apart, and the two pieces"
    );
    assert!(
        worst_cover.0 < 1.0e-9,
        "at ordinate {} the arcs cover {:.9} of a turn between them",
        worst_cover.1,
        worst_cover.0 / TAU + 1.0
    );
}
