use toile_engine::draft::{
    Command, Doc, EdgeRange, Identity, Piece, PieceKey, Point, PointKey, Seam, SeamOrientation,
    Winding,
};

use super::{Cut, PANELS, TAGS};

/// Where each panel is drawn beside the one before it on the table.
pub const BESIDE: f64 = 70.0;

/// Draws the three panels in `order`, and hands them back by panel and not by
/// the place the document keeps them.
///
/// Each panel's own drawing comes from its number, so reordering the file moves
/// no line of the pattern: the left front is 26 cm wide and sits at the same
/// place on the table whether it is written first or last.
pub fn draw(doc: &mut Doc, cut: Cut, order: [usize; 3]) -> Vec<PieceKey> {
    let widths = [cut.front, cut.back, cut.front];
    let mut panels = vec![None; 3];
    for k in order {
        panels[k] = Some(panel(doc, k, widths[k], cut.length));
    }
    panels
        .into_iter()
        .map(|key| key.expect("every panel is drawn once"))
        .collect()
}

/// One rectangular panel, drawn in contour order from its top left corner.
fn panel(doc: &mut Doc, k: usize, width: f64, length: f64) -> PieceKey {
    let (tag, from) = (TAGS[k], BESIDE * k as f64);
    let nodes = [
        (format!("pecho_izq{tag}"), from, 0.0),
        (format!("pecho_der{tag}"), from + width, 0.0),
        (format!("bajo_der{tag}"), from + width, length),
        (format!("bajo_izq{tag}"), from, length),
    ];
    let points: Vec<PointKey> = nodes
        .iter()
        .map(|(label, x, y)| doc.points.insert(Point::at(*x, *y).named(label)))
        .collect();
    doc.pieces
        .insert(Piece::polygon(PANELS[k], points, Winding::Cw))
}

/// Chains the three panels with a seam down each side of the body.
///
/// Each seam joins the right-hand edge of one panel to the left-hand edge of
/// the next. The right edge runs top to bottom in contour order and the left
/// edge runs bottom to top, so the two stretches are opposed — which is how the
/// chest corner of one meets the chest corner of the other rather than its hem.
/// `buttoned` adds the third seam, down the centre front, which closes the
/// strip into a tube.
pub fn sew(doc: &mut Doc, panels: &[PieceKey], buttoned: bool) {
    for (k, j) in [(0, 1), (1, 2)] {
        join(doc, panels, k, j);
    }
    if buttoned {
        join(doc, panels, 2, 0);
    }
}

/// Sews the right-hand edge of panel `k` to the left-hand edge of panel `j`.
fn join(doc: &mut Doc, panels: &[PieceKey], k: usize, j: usize) {
    let (left, right) = (TAGS[k], TAGS[j]);
    stitch(
        doc,
        range(
            doc,
            panels[k],
            &format!("pecho_der{left}"),
            &format!("bajo_der{left}"),
        ),
        range(
            doc,
            panels[j],
            &format!("bajo_izq{right}"),
            &format!("pecho_izq{right}"),
        ),
    );
}

/// Adds one plain seam between two stretches that run against each other.
pub fn stitch(doc: &mut Doc, a: EdgeRange, b: EdgeRange) {
    Command::AddSeam {
        identity: Identity::New,
        seam: Seam::plain(a, b, SeamOrientation::Opposed),
    }
    .apply(doc)
    .expect("both ends of the fixture's seams are nodes it has just named");
}

/// The stretch of one piece's contour between two of its named nodes.
pub fn range(doc: &Doc, piece: PieceKey, head: &str, tail: &str) -> EdgeRange {
    let named = |label| {
        doc.shows_label(piece, label)
            .expect("the fixture names every node it goes on to point at")
    };
    EdgeRange::between(piece, named(head), named(tail))
}
