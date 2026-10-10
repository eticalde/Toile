use toile_doc::{
    Command, Doc, EdgeRange, Hang, Identity, MeasureSet, Piece, PieceKey, Point, PointKey, Seam,
    SeamOrientation, Winding,
};

use super::super::Session;
use crate::body::Collider;

/// A rectangle `wide` by `tall` centimetres drawn from `from`, every node
/// named with `tag` before it so one document can carry several.
fn panel(doc: &mut Doc, tag: &str, from: f64, wide: f64, tall: f64) -> PieceKey {
    let corners = [
        ("arriba_izq", from, 0.0),
        ("arriba_der", from + wide, 0.0),
        ("abajo_der", from + wide, tall),
        ("abajo_izq", from, tall),
    ];
    let points: Vec<PointKey> = corners
        .iter()
        .map(|&(label, x, y)| {
            doc.points
                .insert(Point::at(x, y).named(&format!("{tag}_{label}")))
        })
        .collect();
    doc.pieces.insert(Piece::polygon(tag, points, Winding::Cw))
}

/// The stretch of one piece's contour between two of its named nodes.
fn range(doc: &Doc, piece: PieceKey, tag: &str, head: &str, tail: &str) -> EdgeRange {
    let named = |label: &str| {
        doc.shows_label(piece, &format!("{tag}_{label}"))
            .expect("the fixture names every node it points at")
    };
    EdgeRange::between(piece, named(head), named(tail))
}

/// Closes two panels into a tube: both side edges, each side walked in
/// contour order so the two run the same way.
fn close(doc: &mut Doc, (a, ta): (PieceKey, &str), (b, tb): (PieceKey, &str)) {
    let right = Seam::plain(
        range(doc, a, ta, "arriba_der", "abajo_der"),
        range(doc, b, tb, "arriba_der", "abajo_der"),
        SeamOrientation::Aligned,
    );
    let left = Seam::plain(
        range(doc, a, ta, "abajo_izq", "arriba_izq"),
        range(doc, b, tb, "abajo_izq", "arriba_izq"),
        SeamOrientation::Aligned,
    );
    for seam in [right, left] {
        Command::AddSeam {
            identity: Identity::New,
            seam,
        }
        .apply(doc)
        .expect("both ends of the fixture's seams are nodes it has just named");
    }
}

/// Hangs one panel's top edge from the body's own waist.
fn hang(doc: &mut Doc, piece: PieceKey, tag: &str) {
    Command::AddHang {
        identity: Identity::New,
        hang: Hang::new(
            range(doc, piece, tag, "arriba_izq", "arriba_der"),
            Hang::WAIST,
        ),
    }
    .apply(doc)
    .expect("the top edge runs between two nodes of the piece it names");
}

/// A component sewn only to itself has no reason to be on any ring of a
/// person, even where another piece of the document is the one that was hung.
///
/// The defect in the smallest shape that carries it: a declared panel, and a
/// two-piece bag sewn into a tube and declared nothing. The bag's group holds
/// every piece that carries a seam, so a count of stitches called it "the whole
/// product" and let it go round a hoop of its own girth — measured on the
/// reference body, a 19 cm ring at a calf, with nothing said about it. The
/// declared panel is the only ring of this document, and the bag is counted.
#[test]
fn a_bag_sewn_only_to_itself_is_not_the_product_a_declared_panel_belongs_to() {
    let mut doc = Doc::new(MeasureSet::new("Maniquí", []));
    let held = panel(&mut doc, "panel", 0.0, 40.0, 40.0);
    let front = panel(&mut doc, "bolsa_a", 70.0, 10.0, 12.0);
    let back = panel(&mut doc, "bolsa_b", 90.0, 10.0, 12.0);
    close(&mut doc, (front, "bolsa_a"), (back, "bolsa_b"));
    hang(&mut doc, held, "panel");

    let session = Session::from_doc(doc, Collider::demo()).expect("all three drape");
    assert!(session.seam_faults().is_empty(), "the bag's seams pair");
    let rings = session.rings();
    assert_eq!(rings.len(), 1, "one ring: the one a person declared");
    let placed: Vec<usize> = rings[0]
        .wraps
        .iter()
        .enumerate()
        .filter_map(|(piece, wrap)| wrap.map(|_| piece))
        .collect();
    assert_eq!(placed, [0], "and the panel is the piece on it");
    assert_eq!(session.adrift(), 2, "the bag's two pieces are counted");
}

/// And a garment drawn beside a panel nobody sewed and nobody hung is still
/// the whole product, which is what keeps every golden in the tree where it is.
///
/// The reasonable garment the other half of this rule breaks if the count is
/// taken over every piece on the stand instead: a lone panel on the table has
/// no partner to be placed against and is not waiting to be, and counting it
/// would take a finished tube off the body the moment anybody drew one.
#[test]
fn a_lone_panel_nobody_sewed_leaves_the_garment_beside_it_on_the_body() {
    let mut doc = Doc::new(MeasureSet::new("Maniquí", []));
    let front = panel(&mut doc, "delantero", 0.0, 42.0, 40.0);
    let back = panel(&mut doc, "trasero", 70.0, 42.0, 40.0);
    close(&mut doc, (front, "delantero"), (back, "trasero"));
    panel(&mut doc, "suelta", 140.0, 20.0, 20.0);

    let session = Session::from_doc(doc, Collider::demo()).expect("all three drape");
    let ring = session.layout().expect("the two seams place the tube");
    let placed: Vec<usize> = ring
        .wraps
        .iter()
        .enumerate()
        .filter_map(|(piece, wrap)| wrap.map(|_| piece))
        .collect();
    assert_eq!(placed, [0, 1], "the tube is on the body, the panel is not");
    assert_eq!(session.adrift(), 0, "and nothing of the product is adrift");
}
