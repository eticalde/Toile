use toile_engine::draft::{Doc, MeasureSet, Piece, PieceKey, Point, PointKey, Winding};

use super::shape::{BESIDE, draw, range, sew, stitch};
use super::{CHEST, Cut, hang_by_the_chest};

mod scene;

/// How wide and how deep each half of the bag is drawn, in centimetres.
const WIDE: f64 = 14.0;
const DEEP: f64 = 18.0;

/// Where the bag is drawn across the table, in centimetres: clear of the three
/// panels, of the collar strip and of the band.
const ASIDE: f64 = BESIDE * 7.0;

/// The names the bag's two halves carry in the product tree.
const HALVES: [&str; 2] = ["Bolsa frente", "Bolsa dorso"];

/// A blouse, and beside it a bag of two panels sewn to each other and to
/// nothing else.
///
/// Two components of one document, which is an ordinary thing to have on a
/// table: a patch pocket drawn beside the garment it belongs to, a facing not
/// yet sewn on, the second garment of a set. The blouse declares its chest
/// ring; the bag declares nothing and no seam of it reaches anything that
/// does, which is the whole of the case.
///
/// `station` is the ring the blouse's chest line is hung from, or `None` for
/// the same two components with nothing declared anywhere — the control, and
/// the placement the tree ran before a station could be declared.
pub fn with_a_bag(cut: Cut, station: Option<&str>) -> Doc {
    let mut doc = Doc::new(MeasureSet::new("Maniquí", []));
    let panels = draw(&mut doc, cut, [0, 1, 2]);
    sew(&mut doc, &panels, false);
    let halves = bag(&mut doc);
    bag_seam(&mut doc, &halves);
    if let Some(station) = station {
        hang_by_the_chest(&mut doc, station, None);
    }
    doc
}

/// The blouse of the bench with the bag beside it, declaring the chest.
pub fn declared(cut: Cut) -> Doc {
    with_a_bag(cut, Some(CHEST))
}

/// The bag's two halves, drawn side by side.
fn bag(doc: &mut Doc) -> Vec<PieceKey> {
    HALVES
        .iter()
        .enumerate()
        .map(|(k, name)| half(doc, k, name))
        .collect()
}

/// One half of the bag, drawn clockwise from its top left corner.
fn half(doc: &mut Doc, k: usize, name: &str) -> PieceKey {
    let from = ASIDE + (WIDE + 10.0) * k as f64;
    let nodes = [
        (format!("bolsa_ti_{k}"), from, 0.0),
        (format!("bolsa_td_{k}"), from + WIDE, 0.0),
        (format!("bolsa_bd_{k}"), from + WIDE, DEEP),
        (format!("bolsa_bi_{k}"), from, DEEP),
    ];
    let points: Vec<PointKey> = nodes
        .iter()
        .map(|(label, x, y)| doc.points.insert(Point::at(*x, *y).named(label)))
        .collect();
    doc.pieces.insert(Piece::polygon(name, points, Winding::Cw))
}

/// Sews the bag's two halves down one side, and to nothing else.
///
/// One seam, so the two halves chain into a strip of their own: the component
/// is perfectly placeable and that is exactly the trouble. Left to find a ring
/// by its own girth it went round the ankle.
fn bag_seam(doc: &mut Doc, halves: &[PieceKey]) {
    stitch(
        doc,
        range(doc, halves[0], "bolsa_td_0", "bolsa_bd_0"),
        range(doc, halves[1], "bolsa_bi_1", "bolsa_ti_1"),
    );
}
