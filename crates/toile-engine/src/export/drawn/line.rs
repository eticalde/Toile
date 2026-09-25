use toile_geom::curve;

use super::{Run, anchor};
use crate::draft::{Draft, InternalLine, LineKind, LineSpan, LineVertex, PieceKey};

/// Whether a line of this kind is drawn broken.
///
/// A fold, a stitching line and a line the draft was only reasoned with are
/// drawn broken, the way a paper pattern draws them; what is cut, finished or
/// matched against another piece is drawn whole. The choice is made from what
/// the line is for and never from a stroke the document does not hold — and it
/// is made once, here: a fold a cutter cannot tell from a stitch is a garment
/// cut wrong, and a fold broken on the screen and whole on the paper is two
/// patterns.
fn broken(kind: LineKind) -> bool {
    match kind {
        LineKind::Fold | LineKind::Stitch | LineKind::Reference => true,
        LineKind::Slit | LineKind::Buttonhole | LineKind::Placement => false,
    }
}

/// Every internal line of a piece, as a run of places in centimetres.
///
/// A line one of whose places resolves nowhere is left out rather than drawn
/// short: a run cut off at the last place that resolved would be a line nobody
/// drafted, on a sheet somebody cuts cloth from.
pub(super) fn runs(draft: &Draft, piece: PieceKey) -> Vec<Run> {
    draft
        .doc()
        .lines
        .iter()
        .filter(|(_, held)| held.piece == piece)
        .filter_map(|(_, held)| {
            Some(Run {
                at: run(draft, held)?,
                broken: broken(held.kind),
                label: held.label.clone(),
            })
        })
        .collect()
}

/// The line as a polyline in centimetres, and nothing at all when one of its
/// places resolves nowhere.
fn run(draft: &Draft, held: &InternalLine) -> Option<Vec<[f64; 2]>> {
    let mut out = vec![place(draft, held.head)?];
    for span in &held.spans {
        let from = *out.last().expect("the run opens on its head");
        let to = place(draft, span.to)?;
        if span.segment.bends() {
            let (a, b) = bent(draft, span)?;
            out.extend(
                curve::flatten(from, a, b, to, span.samples)
                    .into_iter()
                    .skip(1),
            );
        }
        out.push(to);
    }
    Some(out)
}

/// Where one place of a line falls, in centimetres.
fn place(draft: &Draft, vertex: LineVertex) -> Option<[f64; 2]> {
    match vertex {
        LineVertex::Contour(at) => Some(anchor::on(draft, &at)?.at),
        LineVertex::Free { point } => draft.resolved(point),
    }
}

/// The two handles a bending span hangs on, in centimetres.
fn bent(draft: &Draft, span: &LineSpan) -> Option<([f64; 2], [f64; 2])> {
    let (out, into) = span.segment.handles()?;
    Some((draft.resolved(out)?, draft.resolved(into)?))
}
