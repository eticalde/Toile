use super::draw::{Bend, bend, binding};
use crate::{
    Command, Doc, EdgeRange, Identity, Piece, PieceKey, Point, PointKey, Seam, SeamOrientation,
    Variable, Winding,
};

/// The quantities the back is drafted on, over the same measurements.
///
/// The back takes the wider share of the hip, scoops a crotch extension twice
/// the front's, rises above the front waistline at the centre back, and cuts
/// its knee and hem four centimetres fuller. `origen_tras` places the whole
/// piece beside the front on the table.
const VARIABLES: [(&str, &str); 7] = [
    ("origen_tras", "45"),
    ("holgura_cadera_tras", "2"),
    ("levante_tras", "2.5"),
    ("extension_tiro_tras", "cadera / 8"),
    ("ancho_rodilla_tras", "ancho_rodilla + 4"),
    ("ancho_bajo_tras", "ancho_bajo + 4"),
    (
        "raya_tras",
        "(cadera / 4 + holgura_cadera_tras - extension_tiro_tras) / 2",
    ),
];

/// The nine nodes of the back, in contour order: name, then x and y.
const NODES: [(&str, &str, &str); 9] = [
    ("cintura_cb", "origen_tras", "-levante_tras"),
    ("cintura_lat_tras", "origen_tras + cintura / 4 + 2", "0"),
    (
        "cadera_lat_tras",
        "origen_tras + cadera / 4 + holgura_cadera_tras",
        "altura_cadera",
    ),
    (
        "rodilla_lat_tras",
        "origen_tras + raya_tras + ancho_rodilla_tras / 2",
        "(tiro + largo_lateral) / 2",
    ),
    (
        "bajo_lat_tras",
        "origen_tras + raya_tras + ancho_bajo_tras / 2",
        "largo_lateral",
    ),
    (
        "bajo_int_tras",
        "origen_tras + raya_tras - ancho_bajo_tras / 2",
        "largo_lateral",
    ),
    (
        "rodilla_int_tras",
        "origen_tras + raya_tras - ancho_rodilla_tras / 2",
        "(tiro + largo_lateral) / 2",
    ),
    ("tiro_int_tras", "origen_tras - extension_tiro_tras", "tiro"),
    ("tiro_cb", "origen_tras", "tiro - extension_tiro_tras"),
];

/// The two tracts the back bends, built the way the front builds its own.
///
/// The back crotch square has twice the front's side, so its arc at
/// twenty-four samples strays the same tenth of a millimetre the front's does
/// at sixteen.
const CURVES: [Bend; 2] = [
    Bend {
        from: "cintura_lat_tras",
        out: (
            "manija_cadera_tras_1",
            "origen_tras + cintura / 4 + 2",
            "altura_cadera / 2",
        ),
        into: (
            "manija_cadera_tras_2",
            "origen_tras + cadera / 4 + holgura_cadera_tras",
            "altura_cadera / 2",
        ),
        samples: 24,
    },
    Bend {
        from: "tiro_int_tras",
        out: (
            "manija_tiro_tras_1",
            "origen_tras - extension_tiro_tras * 0.55",
            "tiro",
        ),
        into: (
            "manija_tiro_tras_2",
            "origen_tras",
            "tiro - extension_tiro_tras * 0.45",
        ),
        samples: 24,
    },
];

/// The name the back carries in the product tree.
pub const BACK: &str = "Trasero";

/// Draws the back into a document the front is already drawn in, and sews the
/// two together at the side and at the inseam.
pub(super) fn trouser_back(doc: &mut Doc) {
    for (name, source) in VARIABLES {
        doc.variables.insert(Variable::new(name, binding(source)));
    }
    let points: Vec<PointKey> = NODES
        .iter()
        .map(|&(label, x, y)| {
            doc.points
                .insert(Point::at(binding(x), binding(y)).named(label))
        })
        .collect();
    let piece = doc.pieces.insert(Piece::polygon(BACK, points, Winding::Cw));
    for curve in &CURVES {
        bend(doc, piece, curve);
    }
    sew(doc, piece);
}

/// Sews the side and the inseam, front to back.
///
/// Both pieces are drawn the same way up and both ranges follow contour
/// order — waist to hem down the side, hem to crotch up the inseam — so the
/// two sides of each seam run the same way and the orientation is aligned.
fn sew(doc: &mut Doc, back: PieceKey) {
    let front = doc
        .piece_named(super::FRONT)
        .expect("the back is drawn into the document the front is drawn in");
    let side = Seam::plain(
        range(doc, front, "cintura_lat", "bajo_lat"),
        range(doc, back, "cintura_lat_tras", "bajo_lat_tras"),
        SeamOrientation::Aligned,
    );
    let inseam = Seam::plain(
        range(doc, front, "bajo_int", "tiro_int"),
        range(doc, back, "bajo_int_tras", "tiro_int_tras"),
        SeamOrientation::Aligned,
    );
    for seam in [side, inseam] {
        Command::AddSeam {
            identity: Identity::New,
            seam,
        }
        .apply(doc)
        .expect("both ends of a block seam are nodes the block has just named");
    }
}

/// The stretch of one piece's contour between two of its named nodes.
fn range(doc: &Doc, piece: PieceKey, head: &str, tail: &str) -> EdgeRange {
    let named = |label| {
        doc.shows_label(piece, label)
            .expect("the block sews between nodes it has just named itself")
    };
    EdgeRange::between(piece, named(head), named(tail))
}
