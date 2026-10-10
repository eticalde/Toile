use toile_engine::draft::{
    Command, Doc, Hang, Heading, Identity, MeasureSet, Piece, PieceKey, Point, PointKey, Sense,
    Winding,
};

use super::shape::{BESIDE, draw, range, sew, stitch};
use super::{CHEST, Cut, TAGS, hang_by_the_chest};

mod scene;

/// The ring a collar strip is hung from.
///
/// The neck, which is where a collar goes and what the catalogue calls it. The
/// strip carries the garment's own girth rather than the neck's, so this is a
/// wide funnel and not a shirt collar — the ring says where the cloth is put,
/// never how much of it there is.
pub const NECK: &str = "cuello";

/// The name the collar strip carries in the product tree.
const COLLAR: &str = "Tira de cuello";

/// How tall the collar strip is drawn, in centimetres.
///
/// The reference body's own rise from its chest ring to its neck ring:
/// `pecho_alto` sits at 0.5853 m and `cuello` at 0.7783, so a strip sewn to the
/// chest line and reaching the neck one is 19.30 cm tall on this person.
/// Drafted to the body, in other words, which is the only way the two
/// declarations and the drawing can agree — a strip drawn shorter would be hung
/// from a ring it cannot reach, and the seam would pay the difference.
const RISE: f64 = 19.30;

/// Where the strip is drawn across the table, in centimetres.
///
/// Past the three panels, so no two pieces of the fixture overlap on the page.
/// It changes nothing about the placement: a piece's own abscissae are read
/// against each other and the offset cancels.
const ASIDE: f64 = BESIDE * 3.0;

/// Where the strip's own centre front stands, in metres of pattern.
///
/// Its lowest abscissa, which is the node its lower edge's last stretch is
/// sewn to: that stretch meets the left front, and the two run against each
/// other, so the strip's low end is the garment's centre front and not its
/// side. The whole of the heading bench's reading of this piece is taken here.
pub(super) fn centre_front() -> f64 {
    ASIDE / 100.0
}

/// Which panel each stretch of the strip's lower edge is sewn to.
///
/// Right to left, because that is the order the contour walks that edge: the
/// strip is drawn clockwise from its top left corner, so its lower edge comes
/// back the other way. Sewn left to right it would be the same three seams with
/// the garment inside out.
const UNDER: [usize; 3] = [2, 1, 0];

/// The same three-panel blouse with a collar strip sewn across all three of its
/// chest lines.
///
/// The garment the whole of this part is about. The strip gives the back panel
/// a third seam, and a chain has room for two, so before a second ring could be
/// placed the strip took the entire product off the body: every piece went to
/// the flat release over the crown.
///
/// `neck` is the ring the strip's own top edge is hung from, or `None` for the
/// same garment with the strip declaring nothing — the control, and the whole
/// of the behaviour the tree had before this. `heading` turns both declared
/// lines, the chest's and the strip's, since a ring that is declared is a ring
/// that is read and the storage order is what is left of the rest.
pub fn collared(cut: Cut, neck: Option<&str>, heading: Option<Heading>) -> Doc {
    let mut doc = Doc::new(MeasureSet::new("Maniquí", []));
    let panels = draw(&mut doc, cut, [0, 1, 2]);
    sew(&mut doc, &panels, false);
    let strip = strip(&mut doc, cut);
    collar_seams(&mut doc, strip, &panels);
    hang_by_the_chest(&mut doc, CHEST, heading);
    if let Some(neck) = neck {
        hang_the_strip(&mut doc, strip, neck, heading);
    }
    doc
}

/// The whole garment declaring both of its rings and facing the centre front.
pub fn facing_the_front(cut: Cut) -> Doc {
    collared(cut, Some(NECK), Some(Heading::facing(0.0, Sense::Leftward)))
}

/// Where each node of the strip's lower edge stands, in centimetres across it.
///
/// Four of them for three seams: a stretch of contour runs between two nodes,
/// so three stretches side by side on one edge want one more node than
/// stretches.
fn joints(cut: Cut) -> [f64; 4] {
    let widths = UNDER.map(|panel| [cut.front, cut.back, cut.front][panel]);
    let mut at = [cut.girth() * 100.0; 4];
    for k in 0..3 {
        at[k + 1] = at[k] - widths[k];
    }
    at
}

/// The collar strip: one long rectangle whose lower edge is cut into the three
/// stretches the three panels are sewn to.
///
/// As long as the three chest lines put together, so the strip carries the
/// garment's own girth and every seam has the same cloth on both sides.
pub(super) fn strip(doc: &mut Doc, cut: Cut) -> PieceKey {
    let girth = cut.girth() * 100.0;
    let mut nodes = vec![
        ("cuello_izq".to_owned(), ASIDE, -RISE),
        ("cuello_der".to_owned(), ASIDE + girth, -RISE),
    ];
    for (k, at) in joints(cut).into_iter().enumerate() {
        nodes.push((format!("base_{k}"), ASIDE + at, 0.0));
    }
    let points: Vec<PointKey> = nodes
        .iter()
        .map(|(label, x, y)| doc.points.insert(Point::at(*x, *y).named(label)))
        .collect();
    doc.pieces
        .insert(Piece::polygon(COLLAR, points, Winding::Cw))
}

/// Sews each stretch of the strip's lower edge to the chest line under it.
///
/// Three seams on one piece, which is the shape this fixture exists for. The
/// strip's lower edge runs right to left in contour order and every chest line
/// runs left to right, so the two are opposed exactly as the side seams are.
pub(super) fn collar_seams(doc: &mut Doc, strip: PieceKey, panels: &[PieceKey]) {
    for (k, panel) in UNDER.into_iter().enumerate() {
        let tag = TAGS[panel];
        stitch(
            doc,
            range(doc, strip, &format!("base_{k}"), &format!("base_{}", k + 1)),
            range(
                doc,
                panels[panel],
                &format!("pecho_izq{tag}"),
                &format!("pecho_der{tag}"),
            ),
        );
    }
}

/// Hangs the strip's own top edge from `neck`, facing `heading`.
///
/// Its top edge and not its lower one: the strip's upper line is the one that
/// has to clear the head, and the lower one is already held by the chest line
/// it is sewn to.
pub(super) fn hang_the_strip(doc: &mut Doc, strip: PieceKey, neck: &str, heading: Option<Heading>) {
    let at = range(doc, strip, "cuello_izq", "cuello_der");
    let hang = match heading {
        Some(heading) => Hang::facing(at, neck, heading),
        None => Hang::new(at, neck),
    };
    Command::AddHang {
        identity: Identity::New,
        hang,
    }
    .apply(doc)
    .expect("the strip's top edge runs between two nodes of the piece it names");
}
