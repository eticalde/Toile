use super::geom::{lerp, sub};
use super::intersect::{self, Crossing};
use super::select::{self, perimeter, x_extent};

/// Scans a horizontal band and keeps the trunk loop with the extreme
/// perimeter: the fullest section if `want_max`, the narrowest otherwise.
/// Offsets where the band has no valid trunk cross-section (below the leg
/// fork, or fused with a limb) are simply skipped, not treated as an error
/// — only the whole band coming up empty is.
///
/// `lo`/`hi`/`step` are the caller's own documented constants: fixing them
/// is part of what a golden then pins. Returns the winning height alongside
/// the ring, since `bajo_pecho` is placed at a fixed offset below wherever
/// `pecho` actually landed.
///
/// # Panics
/// If no offset in `[lo, hi]` yields a valid trunk loop at all.
pub(super) fn trunk_extremum(
    positions: &[[f64; 3]],
    tris: &[[u32; 3]],
    lo: f64,
    hi: f64,
    step: f64,
    want_max: bool,
) -> (Vec<Crossing>, f64) {
    let steps = ((hi - lo) / step).round() as i64;
    let mut best: Option<(Vec<Crossing>, f64, f64)> = None;
    for i in 0..=steps {
        let y = lo + (i as f64) * step;
        let found = intersect::loops(positions, tris, [0.0, y, 0.0], [0.0, 1.0, 0.0]);
        let Some(ring) = select::pick_trunk_loop(positions, &found) else {
            continue;
        };
        let per = perimeter(positions, &ring);
        let better = best.as_ref().is_none_or(|(_, _, best_per)| {
            if want_max {
                per > *best_per
            } else {
                per < *best_per
            }
        });
        if better {
            best = Some((ring, y, per));
        }
    }
    let (ring, y, _) = best.expect("no valid trunk cross-section in the scanned band");
    (ring, y)
}

/// Scans a horizontal band for the widest section by left-right extent,
/// used only for `cabeza` (see [`trunk_extremum`] for the girth version).
///
/// # Panics
/// If no offset in `[lo, hi]` yields a valid trunk loop at all.
pub(super) fn widest_by_extent(
    positions: &[[f64; 3]],
    tris: &[[u32; 3]],
    lo: f64,
    hi: f64,
    step: f64,
) -> Vec<Crossing> {
    let steps = ((hi - lo) / step).round() as i64;
    let mut best: Option<(Vec<Crossing>, f64)> = None;
    for i in 0..=steps {
        let y = lo + (i as f64) * step;
        let found = intersect::loops(positions, tris, [0.0, y, 0.0], [0.0, 1.0, 0.0]);
        let Some(ring) = select::pick_trunk_loop(positions, &found) else {
            continue;
        };
        let extent = x_extent(positions, &ring);
        if best
            .as_ref()
            .is_none_or(|(_, best_extent)| extent > *best_extent)
        {
            best = Some((ring, extent));
        }
    }
    best.map(|(ring, _)| ring)
        .expect("no valid head cross-section in the scanned band")
}

/// Scans downward from `top_y` in fixed steps and returns the height of the
/// first offset that still yields a valid trunk loop — the highest cut that
/// has not yet fused with the arms.
///
/// Used for `pecho_alto`: the anatomical armpit level is not the shoulder
/// joint's own height (the mesh has already fused arm and torso there) but
/// the highest point below it where a horizontal tape could still travel
/// without crossing an arm — see `crate::anny_bake`'s doc.
///
/// # Panics
/// If no offset within 60 tries (0.3 m) yields a valid trunk loop.
pub(super) fn highest_unfused_trunk_y(
    positions: &[[f64; 3]],
    tris: &[[u32; 3]],
    top_y: f64,
) -> f64 {
    const STEP: f64 = 0.005;
    const MAX_TRIES: u32 = 60;
    for i in 0..MAX_TRIES {
        let y = top_y - f64::from(i) * STEP;
        let found = intersect::loops(positions, tris, [0.0, y, 0.0], [0.0, 1.0, 0.0]);
        if select::pick_trunk_loop(positions, &found).is_some() {
            return y;
        }
    }
    panic!("no unfused trunk cross-section found scanning down from y = {top_y}")
}

/// Scans downward from `start_y` in fixed steps and returns the *lowest*
/// height that still yields a valid trunk loop — the fork.
///
/// Below the true fork the two legs are separate loops, neither straddling
/// the body's own x = 0 axis (each sits entirely on one side), so
/// [`select::pick_trunk_loop`] finds nothing there; at or above it the legs
/// have joined into one loop that does straddle. The lowest height where
/// that still holds is, by definition, the fork — the anatomically correct
/// anchor for `crotch`, unlike the pelvis joint (see `crate::anny_bake`'s
/// doc), and the anatomically correct lower bound for the `cadera` band:
/// below the fork a "fuller" reading is really both thighs still pressed
/// together, not the seat.
///
/// # Panics
/// If no offset within 400 tries (0.4 m) yields a valid trunk loop, or if
/// `start_y` itself is already invalid (the caller's `start_y` must be a
/// height already known to be above the fork).
pub(super) fn fork_y(positions: &[[f64; 3]], tris: &[[u32; 3]], start_y: f64) -> f64 {
    const STEP: f64 = 0.001;
    const MAX_TRIES: u32 = 400;
    let mut last_valid = None;
    for i in 0..MAX_TRIES {
        let y = start_y - f64::from(i) * STEP;
        let found = intersect::loops(positions, tris, [0.0, y, 0.0], [0.0, 1.0, 0.0]);
        if select::pick_trunk_loop(positions, &found).is_some() {
            last_valid = Some(y);
        } else {
            return last_valid.expect("start_y must already be above the fork");
        }
    }
    panic!("no fork found scanning down from y = {start_y}")
}

/// Scans along the segment from `a` to `b` for the fullest cross-section
/// perpendicular to it, skipping any offset still fused with the torso (see
/// [`select::pick_limb_loop`]). Returns the winning loop and the fraction
/// `t` it was found at, since the shoulder landmark ring reuses that same
/// `t` — see `crate::anny_bake`'s doc on why.
///
/// # Panics
/// If no offset in `(0, 1)` yields a valid, unfused cross-section.
pub(super) fn limb_fullest(
    positions: &[[f64; 3]],
    tris: &[[u32; 3]],
    a: [f64; 3],
    b: [f64; 3],
) -> (Vec<Crossing>, f64) {
    const STEP: f64 = 0.02;
    const STEPS: u32 = 49;
    let axis = sub(b, a);
    let mut best: Option<(Vec<Crossing>, f64, f64)> = None;
    for i in 1..=STEPS {
        let t = f64::from(i) * STEP;
        let point = lerp(a, b, t);
        let found = intersect::loops(positions, tris, point, axis);
        let Some(ring) = select::pick_limb_loop(positions, &found, point) else {
            continue;
        };
        let per = perimeter(positions, &ring);
        if best.as_ref().is_none_or(|(_, _, best_per)| per > *best_per) {
            best = Some((ring, t, per));
        }
    }
    let (ring, t, _) = best.expect("no valid, unfused cross-section along this limb");
    (ring, t)
}
