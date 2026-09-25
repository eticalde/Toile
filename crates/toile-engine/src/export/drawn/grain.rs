use super::super::units::box_of;
use super::Run;
use crate::draft::{Draft, PieceKey};

/// The share of the shorter side of a piece that its grain line runs for.
const SHARE: f64 = 0.6;

/// One barb of a grain line's arrow: how long it is, in centimetres.
const BARB: f64 = 0.4;

/// How far a barb opens off the shaft, in radians.
const OPENING: f64 = 0.42;

/// The grain line of a piece: the direction the warp runs, arrowed at both
/// ends, in centimetres.
///
/// Drawn from the middle of the piece rather than from a node, because the
/// grain is a property of the cloth under the piece and not of any point on its
/// contour. It is measured on the whole cloth, so a piece drawn against a fold
/// carries one arrow across the piece it is cut as, and not one arrow on each
/// half — the warp does not run two ways.
pub(super) fn runs(draft: &Draft, piece: PieceKey) -> Vec<Run> {
    let Some(held) = draft.doc().pieces.get(piece) else {
        return Vec::new();
    };
    let (centre, span) = middle(draft.cloth_cm(piece));
    let radians = held.grain.radians();
    let along = [radians.cos(), radians.sin()];
    let reach = SHARE * span / 2.0;
    let head = [0, 1].map(|axis| centre[axis] + along[axis] * reach);
    let tail = [0, 1].map(|axis| centre[axis] - along[axis] * reach);
    let mut out = vec![Run::mark(vec![tail, head])];
    out.extend(arrow(head, [-along[0], -along[1]]));
    out.extend(arrow(tail, along));
    out
}

/// The middle of a contour, and the shorter side of the box around it.
fn middle(outline: &[[f64; 2]]) -> ([f64; 2], f64) {
    let (low, high) = box_of(outline.iter().copied());
    let centre = [
        f64::midpoint(low[0], high[0]),
        f64::midpoint(low[1], high[1]),
    ];
    (centre, (high[0] - low[0]).min(high[1] - low[1]))
}

/// The two barbs of an arrow at `tip`, opening back along `back`.
fn arrow(tip: [f64; 2], back: [f64; 2]) -> Vec<Run> {
    [OPENING, -OPENING]
        .into_iter()
        .map(|opening| {
            let (sin, cos) = opening.sin_cos();
            let turned = [back[0] * cos - back[1] * sin, back[0] * sin + back[1] * cos];
            Run::mark(vec![
                tip,
                [0, 1].map(|axis| tip[axis] + turned[axis] * BARB),
            ])
        })
        .collect()
}
