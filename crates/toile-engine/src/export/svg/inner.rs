use std::fmt::Write;

use toile_geom::curve;

use super::{CUT, escape, mm};
use crate::draft::{Draft, InternalLine, LineKind, LineSpan, LineVertex, PieceKey};

/// The weight of a line the cutter does not cut on, in millimetres.
///
/// Thinner than the cut line so that a sheet of paper says at a glance what to
/// cut and what only to mark.
const DRAWN: f64 = CUT * 2.0 / 3.0;

/// The dashes a line of this kind is drawn with, in millimetres, or `None` for
/// a solid one.
///
/// A fold, a stitching line and a line the draft was only reasoned with are
/// drawn broken, the way a paper pattern draws them; what is cut, finished or
/// matched against another piece is drawn whole. The choice is made from what
/// the line is for and never from a stroke the document does not hold.
fn dashes(kind: LineKind) -> Option<&'static str> {
    match kind {
        LineKind::Fold | LineKind::Stitch | LineKind::Reference => Some("4 2"),
        LineKind::Slit | LineKind::Buttonhole | LineKind::Placement => None,
    }
}

/// Every internal line of a piece, each its own path, inside that piece's
/// group.
///
/// A line one of whose places resolves nowhere is left out rather than drawn
/// short: a run cut off at the last place that resolved would be a line nobody
/// drafted, on a sheet somebody cuts cloth from.
///
/// On a piece drawn against a fold every line is drawn twice, once on each
/// half. The sheet is the whole cloth, and a mark drawn on half of it is a mark
/// the cutter finds on one side of a piece that has it on both.
pub(super) fn lines(out: &mut String, draft: &Draft, piece: PieceKey) {
    let cloth = draft.cloth(piece);
    for (_, held) in draft.doc().lines.iter() {
        if held.piece != piece {
            continue;
        }
        let Some(run) = run(draft, held) else {
            continue;
        };
        let dash = dashes(held.kind)
            .map(|pattern| format!(" stroke-dasharray=\"{pattern}\""))
            .unwrap_or_default();
        let title = held
            .label
            .as_deref()
            .map(|name| format!("<title>{}</title>", escape(name)))
            .unwrap_or_default();
        let mirrored = cloth.map(|cloth| {
            run.iter()
                .map(|&at| millimetres(cloth.mirror(centimetres(at))))
                .collect()
        });
        for run in std::iter::once(run).chain(mirrored) {
            path(out, &run, &dash, &title);
        }
    }
}

/// One run as a path on the sheet.
fn path(out: &mut String, run: &[[f64; 2]], dash: &str, title: &str) {
    let mut data = String::new();
    for (rank, &[x, y]) in run.iter().enumerate() {
        let verb = if rank == 0 { 'M' } else { 'L' };
        let _ = write!(data, "{verb} {} {} ", mm(x), mm(y));
    }
    let _ = writeln!(
        out,
        "    <path d=\"{}\" fill=\"none\" stroke=\"#000000\" stroke-width=\"{}\"{dash}>{title}\
         </path>",
        data.trim_end(),
        mm(DRAWN)
    );
}

/// The line as a polyline in millimetres, and nothing at all when one of its
/// places resolves nowhere.
fn run(draft: &Draft, held: &InternalLine) -> Option<Vec<[f64; 2]>> {
    let mut out = vec![place(draft, held.piece, held.head)?];
    for span in &held.spans {
        let from = *out.last().expect("the run opens on its head");
        let to = place(draft, held.piece, span.to)?;
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
    Some(out.into_iter().map(millimetres).collect())
}

/// Where one place of a line falls, in centimetres.
fn place(draft: &Draft, piece: PieceKey, vertex: LineVertex) -> Option<[f64; 2]> {
    match vertex {
        LineVertex::Contour(anchor) => {
            let flat = draft.flat_cm(piece);
            let starts = draft.flat_starts(piece);
            let nodes = draft.points_cm(piece);
            let index = nodes.iter().position(|&(key, _)| key == anchor.from)?;
            let end = *starts.get(index + 1).unwrap_or(&flat.len());
            let tract = flat.get(*starts.get(index)?..end)?;
            let next = nodes[(index + 1) % nodes.len()].1;
            along(tract, next, anchor.t)
        }
        LineVertex::Free { point } => draft.resolved(point),
    }
}

/// The two handles a bending span hangs on, in centimetres.
fn bent(draft: &Draft, span: &LineSpan) -> Option<([f64; 2], [f64; 2])> {
    let (out, into) = span.segment.handles()?;
    Some((draft.resolved(out)?, draft.resolved(into)?))
}

/// The place a fraction of the way along one tract of the contour, measured on
/// the flattening the drawing itself uses.
///
/// `tract` opens on its own node and stops short of the next, so the next
/// node's place closes it; a tract of no length answers with its own start.
fn along(tract: &[[f64; 2]], next: [f64; 2], t: f64) -> Option<[f64; 2]> {
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
            return Some([0, 1].map(|axis| pair[0][axis] + (pair[1][axis] - pair[0][axis]) * part));
        }
        walked += span;
    }
    walk.first().copied()
}

fn millimetres([x, y]: [f64; 2]) -> [f64; 2] {
    [x * super::MM_PER_CM, y * super::MM_PER_CM]
}

/// The way back, for the one question the cloth answers in its own units.
fn centimetres([x, y]: [f64; 2]) -> [f64; 2] {
    [x / super::MM_PER_CM, y / super::MM_PER_CM]
}
