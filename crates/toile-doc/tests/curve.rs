#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_doc::{
    Binding, ChangeClass, Command, Doc, DocError, Handle, History, Piece, PieceKey, Point,
    PointKey, Segment, SegmentEdit, block,
};

fn front(doc: &Doc) -> PieceKey {
    doc.piece_named(block::FRONT).expect("the block draws one")
}

fn node(doc: &Doc, label: &str) -> PointKey {
    doc.shows_label(front(doc), label)
        .unwrap_or_else(|| panic!("the block names {label}"))
}

fn piece(doc: &Doc) -> &Piece {
    doc.pieces.get(front(doc)).expect("the key is live")
}

/// The tract leaving `at`.
fn tract(doc: &Doc, at: PointKey) -> Segment {
    let held = piece(doc);
    let index = held.node_index(at).expect("the contour runs through it");
    held.contour[index].segment
}

/// The two handles of the tract leaving `at`.
fn handles(doc: &Doc, at: PointKey) -> (PointKey, PointKey) {
    tract(doc, at).handles().expect("the tract is a curve")
}

/// How finely the hollowed inseam is flattened.
const SAMPLES: u16 = 12;

/// The edit that hollows the inseam, on two handles the document has yet to
/// give a key to.
///
/// The block already draws the hip and the crotch as curves, so the tract that
/// shows what bending a straight one costs is the inseam above the knee.
fn curve(piece: PieceKey, at: PointKey) -> Command {
    Command::SetSegment {
        piece,
        node: at,
        to: SegmentEdit::cubic(
            Point::at(-2.6, 52.0),
            Point::at(-5.2, 38.0).named("manija_entrepierna"),
        ),
    }
}

/// The edit that says how finely the tract leaving `at` is flattened.
fn sampling(piece: PieceKey, at: PointKey, to: u16) -> Command {
    Command::SetSamples {
        piece,
        node: at,
        to,
    }
}

fn straighten(piece: PieceKey, at: PointKey) -> Command {
    Command::SetSegment {
        piece,
        node: at,
        to: SegmentEdit::Line,
    }
}

/// The block with the inseam sampled for a curve, and the node it leaves.
///
/// The count comes before the handles, which is the order the curve tool
/// emits the two edits in: a tract sampled at one point cannot bend.
fn sampled() -> (Doc, PointKey) {
    let mut doc = block::trouser_front();
    let knee = node(&doc, "rodilla_int");
    sampling(front(&doc), knee, SAMPLES)
        .apply(&mut doc)
        .expect("the contour runs through the knee");
    (doc, knee)
}

/// The block with its inseam hollowed, and the node that tract leaves.
fn curved() -> (Doc, PointKey) {
    let (mut doc, knee) = sampled();
    curve(front(&doc), knee)
        .apply(&mut doc)
        .expect("the tract is sampled for a curve");
    (doc, knee)
}

#[test]
fn curving_a_tract_puts_its_two_handles_into_the_document() {
    let (mut doc, knee) = sampled();
    let before = doc.points.len();
    let applied = curve(front(&doc), knee)
        .apply(&mut doc)
        .expect("the tract is sampled for a curve");

    assert_eq!(applied.class, ChangeClass::Topology);
    assert_eq!(applied.touched, [front(&doc)]);
    assert_eq!(doc.points.len(), before + 2);
    let (out, into) = handles(&doc, knee);
    assert_eq!(
        doc.points.get(out).map(|point| &point.x),
        Some(&Binding::Literal(-2.6))
    );
    assert_eq!(
        doc.points
            .get(into)
            .and_then(|point| point.label.as_deref()),
        Some("manija_entrepierna")
    );
    assert!(piece(&doc).cites(out));
    assert_eq!(piece(&doc).node_index(out), None);
}

#[test]
fn a_handle_is_a_point_that_move_point_moves() {
    let (mut doc, knee) = curved();
    let (out, _) = handles(&doc, knee);
    let applied = Command::MovePoint {
        point: out,
        to: [
            Binding::parse("-extension_tiro + 2").expect("the source parses"),
            Binding::literal(52.0),
        ],
    }
    .apply(&mut doc)
    .expect("a handle is a point like any other");

    // Adjusting a tangent moves the drawing and never the node count, which is
    // what keeps it off the remeshing path.
    assert_eq!(applied.class, ChangeClass::Shape);
    assert_eq!(applied.touched, [front(&doc)]);
    assert_eq!(handles(&doc, knee).0, out);
    let held = doc.points.get(out).expect("the key is live");
    assert_eq!(held.x.source(), "-extension_tiro + 2");
}

#[test]
fn straightening_a_curve_and_undoing_it_restores_the_file_byte_for_byte() {
    let (mut doc, knee) = curved();
    let (out, into) = handles(&doc, knee);
    let before = doc.to_canonical_json();
    let command = straighten(front(&doc), knee);
    let mut history = History::new();

    history.begin("enderezar");
    history.edit(&mut doc, command).expect("the tract is there");
    history.end();
    assert_eq!(tract(&doc, knee), Segment::Line);
    assert!(doc.points.get(out).is_none());
    let straightened = doc.to_canonical_json();
    assert_ne!(straightened, before);

    history.undo(&mut doc).expect("the edit undoes");
    assert_eq!(handles(&doc, knee), (out, into));
    assert_eq!(
        doc.to_canonical_json(),
        before,
        "the handles came back as other points"
    );

    history.redo(&mut doc).expect("the edit redoes");
    assert_eq!(doc.to_canonical_json(), straightened);
    history.undo(&mut doc).expect("and undoes again");
    assert_eq!(doc.to_canonical_json(), before);
}

#[test]
fn curving_a_tract_and_undoing_it_leaves_the_contour_as_it_was() {
    let (mut doc, knee) = sampled();
    let before = doc.points.len();
    let issued = doc.points.issued();
    let command = curve(front(&doc), knee);
    let mut history = History::new();

    history.begin("curvar");
    history
        .edit(&mut doc, command)
        .expect("the contour runs through the knee");
    history.end();
    history.undo(&mut doc).expect("the edit undoes");

    assert_eq!(tract(&doc, knee), Segment::Line);
    assert_eq!(doc.points.len(), before);
    // The two slots the handles opened stay open: an index is never handed out
    // twice, so redoing the curve takes its own keys back rather than someone
    // else's.
    assert_eq!(doc.points.issued(), issued + 2);
    history.redo(&mut doc).expect("the edit redoes");
    let (out, into) = handles(&doc, knee);
    assert_eq!((out.index(), into.index()), (issued, issued + 1));
}

#[test]
fn the_sampling_of_a_tract_is_its_own_field() {
    let (mut doc, knee) = curved();
    let before = doc.to_canonical_json();
    let applied = sampling(front(&doc), knee, 24)
        .apply(&mut doc)
        .expect("the tract is there");

    assert_eq!(applied.class, ChangeClass::Topology);
    let index = piece(&doc).node_index(knee).expect("it is a node");
    assert_eq!(piece(&doc).contour[index].samples, 24);
    // Curving the tract left the sampling alone, so taking the count back
    // lands on the one the tract was bent at.
    applied.inverse.apply(&mut doc).expect("the tract is there");
    assert_eq!(doc.to_canonical_json(), before);
}

#[test]
fn a_handle_asking_for_a_key_it_cannot_have_is_refused() {
    let (mut doc, knee) = sampled();
    let piece = front(&doc);
    let waist = node(&doc, "cintura_lat");
    let spare = doc.points.insert(Point::at(0.0, 0.0));
    doc.points.remove(spare).expect("it was just inserted");
    let before = doc.to_canonical_json();
    let asking = |out: Handle, into: Handle| Command::SetSegment {
        piece,
        node: knee,
        to: SegmentEdit::curve(out, into),
    };
    let lost = PointKey::new(40, 0);
    let cases = [
        (
            asking(
                Handle::restored(waist, Point::at(0.0, 0.0)),
                Handle::new(Point::at(1.0, 1.0)),
            ),
            DocError::occupied(waist),
        ),
        (
            asking(
                Handle::restored(spare, Point::at(0.0, 0.0)),
                Handle::restored(spare, Point::at(1.0, 1.0)),
            ),
            DocError::occupied(spare),
        ),
        (
            asking(
                Handle::new(Point::at(0.0, 0.0)),
                Handle::restored(lost, Point::at(1.0, 1.0)),
            ),
            DocError::stale(lost),
        ),
    ];
    for (command, want) in cases {
        assert_eq!(command.clone().apply(&mut doc), Err(want), "{command:?}");
    }
    // A refused edit is a document that did not move.
    assert_eq!(doc.to_canonical_json(), before);
}

#[test]
fn a_tract_the_contour_does_not_have_is_an_error_not_a_panic() {
    let mut doc = block::trouser_front();
    let piece = front(&doc);
    let stray = doc.points.insert(Point::at(0.0, 0.0));
    let commands = [
        straighten(piece, stray),
        Command::SetSamples {
            piece,
            node: stray,
            to: 8,
        },
    ];
    for command in commands {
        assert_eq!(
            command.clone().apply(&mut doc),
            Err(DocError::NoSuchNode),
            "{command:?}"
        );
    }
    let gone = PieceKey::new(9, 0);
    assert_eq!(
        Command::SetSamples {
            piece: gone,
            node: stray,
            to: 8,
        }
        .apply(&mut doc),
        Err(DocError::stale(gone))
    );
}
