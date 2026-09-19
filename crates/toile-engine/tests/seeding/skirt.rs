use toile_engine::draft::{
    Command, Doc, EdgeRange, Elastic, Identity, MeasureSet, Piece, PieceKey, Point, PointKey, Seam,
    SeamOrientation, Winding,
};

/// Each half's waist, in centimetres.
///
/// Two halves make 88 cm of cloth, which is the reference body's own waist to
/// within a fifth of a millimetre: the band is the size of the person, and
/// what holds it on is the ratio it is pulled to, never a garment drawn small.
const WAIST: f64 = 44.0;

/// Each half's hip: the reference body's 103 cm and five centimetres of ease.
///
/// The ease is what makes the skirt wearable at the waist at all. A garment is
/// only hung where everything it would pass on the way down is narrower than
/// it is, and the hip is the one thing between a waistband and the floor.
const HIP: f64 = 54.0;

/// How far below the waist the hip sits.
const HIP_DROP: f64 = 20.0;

/// Waist to hem: a skirt that ends above the knee, so its own weight hangs
/// clear of the ground and nothing but the band can be holding it.
const LENGTH: f64 = 55.0;

/// Where the back is drawn beside the front on the table.
const BESIDE: f64 = 70.0;

/// The name the front carries in the product tree.
pub const FRONT: &str = "Delantero";

/// The name the back carries.
pub const BACK: &str = "Trasero";

/// A tube skirt over the reference body: a front, a back, the two seams that
/// close them into a tube, and a waistband when one is asked for.
///
/// The fixture the elastic is proved on, and it has to be a fixture: the
/// shipped block is one leg of a trouser, so an elastic at the top of it has a
/// narrowing thigh underneath and nothing to grip, and a whole trouser is a Y
/// that the placement rule does not model. A skirt is the smallest garment
/// that puts a band where the body is wider below it.
///
/// `band` is the ratio the waist is held to and how hard, or `None` for the
/// same skirt with nothing holding it — the control.
pub fn skirt(band: Option<(f64, f64)>) -> Doc {
    let mut doc = Doc::new(MeasureSet::new("Maniquí", []));
    let front = half(&mut doc, FRONT, 0.0, "");
    let back = half(&mut doc, BACK, BESIDE, "_tras");
    sew(&mut doc, front, back);
    if let Some((ratio, strength)) = band {
        for (piece, tag) in [(front, ""), (back, "_tras")] {
            hold(&mut doc, piece, tag, ratio, strength);
        }
    }
    doc
}

/// One half of the skirt: five nodes, drawn from `from` rightwards.
///
/// Straight from the hip to the hem, so the piece is at its full width over
/// the whole of that run and the garment's widest girth is unambiguous.
fn half(doc: &mut Doc, name: &str, from: f64, tag: &str) -> PieceKey {
    let nodes = [
        (format!("cintura_cf{tag}"), from, 0.0),
        (format!("cintura_lat{tag}"), from + WAIST, 0.0),
        (format!("cadera_lat{tag}"), from + HIP, HIP_DROP),
        (format!("bajo_lat{tag}"), from + HIP, LENGTH),
        (format!("bajo_cf{tag}"), from, LENGTH),
    ];
    let points: Vec<PointKey> = nodes
        .iter()
        .map(|(label, x, y)| doc.points.insert(Point::at(*x, *y).named(label)))
        .collect();
    doc.pieces.insert(Piece::polygon(name, points, Winding::Cw))
}

/// Closes the two halves into a tube: the side seam and the centre seam.
///
/// Both halves are drawn the same way up and both sides of each seam follow
/// contour order — waist to hem down the side, hem to waist up the centre — so
/// each seam's two sides run the same way and the orientation is aligned.
fn sew(doc: &mut Doc, front: PieceKey, back: PieceKey) {
    let side = Seam::plain(
        range(doc, front, "cintura_lat", "bajo_lat"),
        range(doc, back, "cintura_lat_tras", "bajo_lat_tras"),
        SeamOrientation::Aligned,
    );
    let centre = Seam::plain(
        range(doc, front, "bajo_cf", "cintura_cf"),
        range(doc, back, "bajo_cf_tras", "cintura_cf_tras"),
        SeamOrientation::Aligned,
    );
    for seam in [side, centre] {
        Command::AddSeam {
            identity: Identity::New,
            seam,
        }
        .apply(doc)
        .expect("both ends of the fixture's seams are nodes it has just named");
    }
}

/// Puts a waistband on one half: the whole of its top edge, held in.
fn hold(doc: &mut Doc, piece: PieceKey, tag: &str, ratio: f64, strength: f64) {
    let at = range(
        doc,
        piece,
        &format!("cintura_cf{tag}"),
        &format!("cintura_lat{tag}"),
    );
    Command::AddElastic {
        identity: Identity::New,
        elastic: Elastic::new(at, ratio, strength),
    }
    .apply(doc)
    .expect("the band runs between two nodes of the piece it names");
}

/// The stretch of one piece's contour between two of its named nodes.
fn range(doc: &Doc, piece: PieceKey, head: &str, tail: &str) -> EdgeRange {
    let named = |label| {
        doc.shows_label(piece, label)
            .expect("the fixture names every node it goes on to point at")
    };
    EdgeRange::between(piece, named(head), named(tail))
}
