use toile_engine::draft::{
    Command, Doc, Hang, Heading, Identity, MeasureSet, Piece, PieceKey, Point, PointKey, Sense,
    Winding,
};

use super::shape::{BESIDE, draw, range, sew, stitch};
use super::{CHEST, Cut, TAGS, hang_by_the_chest};

mod scene;

/// The ring the band is hung from.
pub const HIP: &str = "cadera";

/// The name the band carries in the product tree.
pub const BAND: &str = "Pretina";

/// How deep the band is drawn, in centimetres.
const DEEP: f64 = 6.0;

/// A blouse whose hem lands on the reference body's hip ring.
///
/// 38.13 cm from the chest line, which is that body's own rise from
/// `pecho_alto` at 0.5853 m to `cadera` at 0.2040: drafted to the person, so
/// the band's own ring and the hem it is sewn to meet at one height. Any other
/// length would declare two rings the drawing does not put that far apart, and
/// the seam between them would carry the difference.
pub const TO_THE_HIP: Cut = Cut {
    front: 26.0,
    back: 52.0,
    length: 38.13,
};

/// Where the band is drawn across the table, in centimetres: clear of the three
/// panels and of the collar strip.
const ASIDE: f64 = BESIDE * 5.0;

/// The same three-panel blouse with a band sewn across all three of its hems.
///
/// The owner's `pretina`, and the same defect as the collar strip read the
/// other way up: the band gives every panel a second seam and itself a third,
/// so the chain gives up and the whole product goes to the flat release. Unlike
/// the collar this ring is below the chest, where nothing of the body stands
/// out sideways, so it is the scene that says what the placement does when the
/// person under it is not in the way.
pub fn banded(cut: Cut, hip: Option<&str>, heading: Option<Heading>) -> Doc {
    let mut doc = Doc::new(MeasureSet::new("Maniquí", []));
    let panels = draw(&mut doc, cut, [0, 1, 2]);
    sew(&mut doc, &panels, false);
    let band = band(&mut doc, cut);
    band_seams(&mut doc, band, &panels);
    hang_by_the_chest(&mut doc, CHEST, heading);
    if let Some(hip) = hip {
        hang_the_band(&mut doc, band, hip, heading);
    }
    doc
}

/// The whole garment declaring both of its rings and facing the centre front.
pub fn facing_the_front(cut: Cut) -> Doc {
    banded(cut, Some(HIP), Some(Heading::facing(0.0, Sense::Leftward)))
}

/// Where the band's own centre front stands, in metres of pattern.
///
/// Its lowest abscissa, for the reason the strip's is its lowest: the first
/// stretch of its upper edge is sewn to the left front's hem, opposed, so the
/// band's low end meets the garment's centre front.
pub(super) fn centre_front() -> f64 {
    ASIDE / 100.0
}

/// Where each node of the band's upper edge stands, in centimetres across it.
fn joints(cut: Cut) -> [f64; 4] {
    let widths = [cut.front, cut.back, cut.front];
    let mut at = [0.0; 4];
    for k in 0..3 {
        at[k + 1] = at[k] + widths[k];
    }
    at
}

/// The band: one long rectangle whose upper edge is cut into the three
/// stretches the three hems are sewn to.
pub(super) fn band(doc: &mut Doc, cut: Cut) -> PieceKey {
    let girth = cut.girth() * 100.0;
    let (top, base) = (cut.length, cut.length + DEEP);
    let mut nodes: Vec<(String, f64, f64)> = joints(cut)
        .into_iter()
        .enumerate()
        .map(|(k, at)| (format!("alto_{k}"), ASIDE + at, top))
        .collect();
    nodes.push(("bajo_der".to_owned(), ASIDE + girth, base));
    nodes.push(("bajo_izq".to_owned(), ASIDE, base));
    let points: Vec<PointKey> = nodes
        .iter()
        .map(|(label, x, y)| doc.points.insert(Point::at(*x, *y).named(label)))
        .collect();
    doc.pieces.insert(Piece::polygon(BAND, points, Winding::Cw))
}

/// Sews each stretch of the band's upper edge to the hem over it.
///
/// The band's upper edge runs left to right in contour order and every hem runs
/// right to left, so the two are opposed exactly as the side seams are.
pub(super) fn band_seams(doc: &mut Doc, band: PieceKey, panels: &[PieceKey]) {
    for (k, tag) in TAGS.iter().enumerate() {
        stitch(
            doc,
            range(doc, band, &format!("alto_{k}"), &format!("alto_{}", k + 1)),
            range(
                doc,
                panels[k],
                &format!("bajo_der{tag}"),
                &format!("bajo_izq{tag}"),
            ),
        );
    }
}

/// Hangs the band's own upper edge from `hip`, facing `heading`.
///
/// Its upper edge, which is the line that holds a band up and the line it
/// shares with the hem it is sewn to: a band hung by its lower edge would be
/// held a band's depth under the garment it carries.
pub(super) fn hang_the_band(doc: &mut Doc, band: PieceKey, hip: &str, heading: Option<Heading>) {
    let at = range(doc, band, "alto_0", "alto_3");
    let hang = match heading {
        Some(heading) => Hang::facing(at, hip, heading),
        None => Hang::new(at, hip),
    };
    Command::AddHang {
        identity: Identity::New,
        hang,
    }
    .apply(doc)
    .expect("the band's upper edge runs between two nodes of the piece it names");
}
