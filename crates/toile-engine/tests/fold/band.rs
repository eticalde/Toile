use toile_engine::draft::{
    Binding, Command, Doc, Draft, EdgeRange, Identity, LineEdit, LineKind, MeasureSet, Piece,
    PieceKey, Point, PointKey, Symmetry, Variable, VertexEdit, Winding,
};

/// The waistband of a baggy-jeans draft, as its own formulas write it.
///
/// Half a band on the fold at centre back: four centimetres of button extension
/// out to `wb_cf`, then half the waistband girth from there to `wb_cb`. Drawn
/// clockwise on the page, four centimetres deep, with the bottom edge split at
/// the side seam the way the draft splits it.
///
/// The names are the draft's own, so the fixture is the piece and not a
/// rectangle standing in for one: what folding it does to the width is the
/// number a person reads off the panel.
const BAND: [(&str, &str, &str); 6] = [
    ("wb_1", "36", "7"),
    ("wb_2", "40 + wbg / 2", "7"),
    ("wb_cb", "40 + wbg / 2", "11"),
    ("wb_side", "40 + wbg / 4", "11"),
    ("wb_cf", "40", "11"),
    ("wb_0", "36", "11"),
];

/// The waistband girth the draft resolves the band against, in centimetres.
pub const GIRTH: f64 = 87.0;

/// The piece as a document, and the key it took.
fn band() -> (Doc, PieceKey) {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 78.0)]));
    doc.variables.insert(Variable::new("wbg", GIRTH));
    let nodes: Vec<PointKey> = BAND
        .iter()
        .map(|&(label, x, y)| {
            let at = Point {
                x: parse(x),
                y: parse(y),
                label: Some(label.to_owned()),
                label_visible: false,
            };
            doc.points.insert(at)
        })
        .collect();
    let piece = doc
        .pieces
        .insert(Piece::polygon("PRETINA", nodes, Winding::Cw));
    (doc, piece)
}

fn parse(source: &str) -> Binding {
    Binding::parse(source).expect("the draft's own formulas parse")
}

/// One belt-loop mark on the band: a placement line across its depth.
fn mark(doc: &mut Doc, piece: PieceKey) {
    let place = |y| VertexEdit::Free {
        identity: Identity::New,
        value: Point::at(50.0, y),
    };
    let edit = LineEdit::new(piece, LineKind::Placement, place(7.0)).to(place(11.0));
    Command::AddLine {
        identity: Identity::New,
        line: Box::new(edit),
    }
    .apply(doc)
    .expect("a place of its own needs nothing of the contour");
}

/// The node a label names on the band.
pub fn node(doc: &Doc, piece: PieceKey, label: &str) -> PointKey {
    doc.shows_label(piece, label).expect("the band names it")
}

/// The band as a document, folded on its centre-back edge or drawn as it is,
/// with one belt-loop mark on it either way.
pub fn written(folded: bool) -> (Doc, PieceKey) {
    let (mut doc, piece) = band();
    mark(&mut doc, piece);
    if folded {
        let axis = EdgeRange::between(piece, node(&doc, piece, "wb_2"), node(&doc, piece, "wb_cb"));
        Command::AddSymmetry {
            identity: Identity::New,
            symmetry: Symmetry::fold(axis),
        }
        .apply(&mut doc)
        .expect("both ends are nodes of the band");
    }
    (doc, piece)
}

/// The same band resolved onto the table.
pub fn drafted(folded: bool) -> (Draft, PieceKey) {
    let (doc, piece) = written(folded);
    (Draft::from_doc(doc).expect("the band resolves"), piece)
}

/// How wide a run of places is, in whatever unit it is written in.
pub fn width(at: &[[f64; 2]]) -> f64 {
    let high = at.iter().map(|p| p[0]).fold(f64::MIN, f64::max);
    let low = at.iter().map(|p| p[0]).fold(f64::MAX, f64::min);
    high - low
}
