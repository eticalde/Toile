#![allow(missing_docs, reason = "a test crate publishes no API surface")]

mod refused;

use toile_doc::{
    Binding, ChangeClass, Command, ContourNode, Dart, DartKey, Doc, DrawnWedge, EdgeRange,
    FORMAT_VERSION, FORMAT_VERSION_DARTED, FoldDirection, History, Identity, PieceKey, Point,
    PointKey, SeamKey, SeamOrientation, SegmentEdit, block,
};

/// The name the gesture that puts a dart on a drawn wedge carries.
const DECLARE: &str = "pinza";

/// A wedge drawn into the back waistline by formula, the way a draft writes
/// one.
///
/// Every coordinate is an expression over the pattern's own quantities, which
/// is what a drafted back has and what no cut can produce: a cut writes the
/// place the pointer landed on. They go in after `cintura_cb`, on the straight
/// tract that runs to the side waist.
const DRAWN: [(&str, &str, &str); 3] = [
    ("pinza_a", "origen_tras + 5", "-levante_tras / 2"),
    ("pinza_pico", "origen_tras + 6", "altura_cadera / 2"),
    ("pinza_b", "origen_tras + 7", "-levante_tras / 4"),
];

/// Three more nodes after those, with the first and the last bound to one
/// place.
const LEVEL: [(&str, &str, &str); 3] = [
    ("llana_a", "origen_tras + 9", "0"),
    ("llana_pico", "origen_tras + 10", "altura_cadera / 3"),
    ("llana_b", "origen_tras + 9", "0"),
];

/// A second wedge, drawn beside the first and as good as it is.
const BESIDE: [(&str, &str, &str); 3] = [
    ("otra_a", "origen_tras + 12", "0"),
    ("otra_pico", "origen_tras + 13", "altura_cadera / 4"),
    ("otra_b", "origen_tras + 14", "0"),
];

/// The shipped block with that wedge drawn into its back, and the three keys.
fn drafted() -> (Doc, PieceKey, [PointKey; 3]) {
    let mut doc = block::trousers();
    let piece = doc.piece_named(block::BACK).expect("the block draws one");
    let after = doc
        .shows_label(piece, "cintura_cb")
        .expect("the block names it");
    let nodes = drawn_in(&mut doc, piece, after, DRAWN);
    (doc, piece, nodes)
}

/// Three nodes written into `piece` one after another, from `after` onward.
pub(crate) fn drawn_in(
    doc: &mut Doc,
    piece: PieceKey,
    after: PointKey,
    write: [(&str, &str, &str); 3],
) -> [PointKey; 3] {
    let mut after = after;
    let mut made = Vec::with_capacity(write.len());
    for (label, x, y) in write {
        Command::InsertNode {
            piece,
            after: Some(after),
            identity: Identity::New,
            value: Point::at(binding(x), binding(y)).named(label),
            segment: SegmentEdit::Line,
            samples: 1,
        }
        .apply(doc)
        .expect("the node it follows is on the waistline");
        after = doc
            .shows_label(piece, label)
            .expect("the insertion named it");
        made.push(after);
    }
    [made[0], made[1], made[2]]
}

fn binding(source: &str) -> Binding {
    Binding::parse(source).expect("the block's own grammar")
}

/// The dart a fresh declaration carries: only its fold is read, because every
/// key it names the wedge already holds or the edit itself issues.
pub(crate) fn asked() -> Dart {
    Dart {
        apex: PointKey::new(0, 0),
        legs: (PointKey::new(0, 0), PointKey::new(0, 0)),
        seam: SeamKey::new(0, 0),
        fold: FoldDirection::TowardStart,
    }
}

pub(crate) fn declare(piece: PieceKey, nodes: [PointKey; 3]) -> Command {
    Command::DeclareDart {
        identity: Identity::New,
        dart: asked(),
        wedge: DrawnWedge { piece, nodes },
    }
}

pub(crate) fn contour(doc: &Doc, piece: PieceKey) -> Vec<ContourNode> {
    doc.pieces
        .get(piece)
        .expect("the key is live")
        .contour
        .clone()
}

pub(crate) fn only_dart(doc: &Doc) -> (DartKey, Dart) {
    let (key, held) = doc.darts.iter().next().expect("the declaration wrote one");
    (key, *held)
}

/// What each of the three nodes is bound to, as the file spells it.
pub(crate) fn sources(doc: &Doc, nodes: [PointKey; 3]) -> Vec<(String, String)> {
    nodes
        .iter()
        .map(|&key| {
            let held = doc.points.get(key).expect("the node is live");
            (held.x.source().into_owned(), held.y.source().into_owned())
        })
        .collect()
}

/// The declaration puts a seam and a record on a drawn wedge, and writes
/// nothing else at all.
#[test]
fn a_drawn_wedge_is_sewn_shut_and_recorded_without_the_contour_moving() {
    let (mut doc, piece, nodes) = drafted();
    let was = contour(&doc, piece);
    let bound = sources(&doc, nodes);
    let points = doc.points.len();
    let applied = declare(piece, nodes)
        .apply(&mut doc)
        .expect("the three stand together on the waistline");
    assert_eq!(applied.touched, vec![piece]);
    assert_eq!(applied.class, ChangeClass::Topology);
    assert_eq!(declare(piece, nodes).class(), ChangeClass::Topology);

    let (key, held) = only_dart(&doc);
    assert_eq!(held.legs, (nodes[0], nodes[2]));
    assert_eq!(held.apex, nodes[1]);
    assert_eq!(held.fold, FoldDirection::TowardStart);
    assert_eq!(contour(&doc, piece), was, "node for node");
    assert_eq!(doc.points.len(), points, "and not one point was placed");
    assert_eq!(sources(&doc, nodes), bound, "nor one binding rewritten");

    let sewn = doc
        .seams
        .get(held.seam)
        .expect("the declaration issued one");
    assert_eq!(sewn.a, EdgeRange::between(piece, nodes[0], nodes[1]));
    assert_eq!(sewn.b, EdgeRange::between(piece, nodes[1], nodes[2]));
    assert_eq!(
        sewn.orientation,
        SeamOrientation::Opposed,
        "the two sides meet at the apex"
    );
    assert_eq!(doc.seams.len(), 3, "the block's two, and the dart's");
    assert_eq!(applied.inverse, Command::UndeclareDart { dart: key });
}

/// Taking the declaration off leaves the wedge exactly as it was drawn: the
/// three nodes in the contour, each bound to the expression it was written
/// with.
///
/// This is the whole of why the declaration exists. A pattern drafted by
/// formula loses its draft the moment anything rewrites one of those
/// coordinates, and the route that was open before this — three nodes deleted
/// and cut again — does exactly that, since a cut writes the place the pointer
/// landed on.
#[test]
fn undeclaring_leaves_the_wedge_drawn_with_its_formulas_intact() {
    let (mut doc, piece, nodes) = drafted();
    let was = contour(&doc, piece);
    let bound = sources(&doc, nodes);
    let seams = doc.seams.len();
    let applied = declare(piece, nodes)
        .apply(&mut doc)
        .expect("the three stand together on the waistline");
    let (key, held) = only_dart(&doc);

    let back = applied.inverse.apply(&mut doc).expect("the dart is live");
    assert_eq!(back.touched, vec![piece]);
    assert_eq!(back.class, ChangeClass::Topology);
    assert!(doc.darts.is_empty());
    assert_eq!(doc.seams.len(), seams, "the dart's seam went with it");
    assert_eq!(contour(&doc, piece), was, "node for node");
    assert_eq!(
        sources(&doc, nodes),
        bound,
        "and every coordinate is the expression it was drawn as"
    );
    assert_eq!(
        bound[1],
        ("origen_tras + 6".to_owned(), "altura_cadera / 2".to_owned()),
        "the apex above all: a place inside the cloth, stated as a formula"
    );
    assert_eq!(
        back.inverse,
        Command::DeclareDart {
            identity: Identity::Restored(key),
            dart: held,
            wedge: DrawnWedge { piece, nodes },
        }
    );
}

/// The whole entry through the history and back: the keys the declaration
/// issued come back the same, and the file it writes is the file it wrote.
///
/// An undo does not take the file back to the very bytes it was, and no edit
/// that creates anything does: the two slots the declaration opened stay
/// counted, which is what stops a reopened document from handing out a key a
/// live seam still holds.
#[test]
fn the_declaration_survives_a_save_an_undo_and_a_redo() {
    let (mut doc, piece, nodes) = drafted();
    let before = doc.to_canonical_json();
    let mut history = History::new();
    history.begin(DECLARE);
    history
        .edit(&mut doc, declare(piece, nodes))
        .expect("the three stand together on the waistline");
    history.end();
    let (key, held) = only_dart(&doc);

    let written = doc.to_canonical_json();
    let read = Doc::from_json(&written).expect("what the writer wrote, the reader reads");
    assert_eq!(read, doc);
    assert_eq!(read.to_canonical_json(), written);

    history.undo(&mut doc).expect("the dart is live");
    let after = doc.to_canonical_json();
    assert_eq!(after.lines().count(), before.lines().count());
    let moved: Vec<&str> = after
        .lines()
        .zip(before.lines())
        .filter(|(now, was)| now != was)
        .map(|(now, _)| now.trim())
        .collect();
    assert!(!moved.is_empty(), "the declaration opened two slots");
    assert!(
        moved.iter().all(|line| line.starts_with("\"issued\"")),
        "{moved:?}"
    );

    history.redo(&mut doc).expect("both slots are open again");
    let (again, back) = only_dart(&doc);
    assert_eq!(again, key, "the dart's own key");
    assert_eq!(back, held, "and the record it wrote");
    assert_eq!(doc.to_canonical_json(), written, "byte for byte");
}

/// A declared dart is the same `Dart` record a cut one is, so it asks for the
/// version a cut one asks for and the format gains no number.
///
/// Checked rather than asserted in prose: the stamp is a pure function of what
/// the document carries, and a declaration carries a dart and a seam — two
/// things version 9 already spells, and a version-9 reader loses neither.
#[test]
fn a_declared_dart_asks_for_the_version_a_cut_one_asks_for() {
    let (mut doc, piece, nodes) = drafted();
    assert_eq!(
        doc.format_version(),
        FORMAT_VERSION,
        "a wedge is just nodes"
    );
    declare(piece, nodes).apply(&mut doc).expect("it fits");
    assert_eq!(doc.format_version(), FORMAT_VERSION_DARTED);
    let written = doc.to_canonical_json();
    assert!(written.starts_with("{\n  \"toile\": 9,"), "{written}");
    assert!(written.contains("\"fold\": \"toward_start\""), "{written}");
    assert_eq!(
        Doc::from_json(&written).expect("this build reads it"),
        doc,
        "and the record reads back as the one that was written"
    );
}
