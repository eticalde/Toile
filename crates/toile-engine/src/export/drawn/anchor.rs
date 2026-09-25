use crate::draft::{Draft, EdgeAnchor};

/// Where a place anchored to a contour falls, and which way that contour runs
/// there.
pub(super) struct Place {
    /// Where it falls, in centimetres.
    pub(super) at: [f64; 2],
    /// Which way the contour runs there, as a unit vector — or nothing, when
    /// the tract it fell on has no length to have a direction.
    pub(super) along: Option<[f64; 2]>,
}

/// Where an anchor falls on the piece it names.
///
/// Read off the flattening the drawing itself uses, tract by tract, so that a
/// mark a fraction of the way along a curved tract lands where the drawing
/// bends and not where its chord would have put it. `None` when the piece or
/// the node the anchor names is not there to be measured along.
pub(super) fn on(draft: &Draft, at: &EdgeAnchor) -> Option<Place> {
    let flat = draft.flat_cm(at.piece);
    let starts = draft.flat_starts(at.piece);
    let nodes = draft.points_cm(at.piece);
    let index = nodes.iter().position(|&(key, _)| key == at.from)?;
    let end = *starts.get(index + 1).unwrap_or(&flat.len());
    let tract = flat.get(*starts.get(index)?..end)?;
    let next = nodes[(index + 1) % nodes.len()].1;
    along(tract, next, at.t)
}

/// The place a fraction of the way along one tract of the contour, measured on
/// the flattening the drawing itself uses.
///
/// `tract` opens on its own node and stops short of the next, so the next
/// node's place closes it; a tract of no length answers with its own start and
/// with no direction, there being no step to take a direction from.
fn along(tract: &[[f64; 2]], next: [f64; 2], t: f64) -> Option<Place> {
    let mut walk: Vec<[f64; 2]> = tract.to_vec();
    walk.push(next);
    let step = |a: [f64; 2], b: [f64; 2]| (b[0] - a[0]).hypot(b[1] - a[1]);
    let total: f64 = walk.windows(2).map(|pair| step(pair[0], pair[1])).sum();
    let wanted = t.clamp(0.0, 1.0) * total;
    let mut walked = 0.0;
    for pair in walk.windows(2) {
        let span = step(pair[0], pair[1]);
        if span <= f64::EPSILON {
            continue;
        }
        if wanted <= walked + span {
            let part = (wanted - walked) / span;
            return Some(Place {
                at: [0, 1].map(|axis| pair[0][axis] + (pair[1][axis] - pair[0][axis]) * part),
                along: Some([0, 1].map(|axis| (pair[1][axis] - pair[0][axis]) / span)),
            });
        }
        walked += span;
    }
    Some(Place {
        at: walk.first().copied()?,
        along: None,
    })
}
