#![allow(missing_docs, reason = "a test crate publishes no API surface")]

/// The two doors out of a dart: the one that keeps its wedge drawn, and what
/// the one that takes the wedge away will not do.
mod loose;
/// What a wedge refuses, and every edit that would leave a dart the contour
/// no longer describes.
mod refused;

use toile_doc::{
    ChangeClass, Command, ContourNode, Dart, DartKey, DartWedge, Doc, DocError, EdgeAnchor,
    EdgeRange, FoldDirection, History, Identity, Notch, PieceKey, Point, PointKey, SeamKey,
    SeamOrientation, SegmentEdit, WedgeNode, block,
};

/// The name the gesture that cuts a dart carries into the status bar.
const CUT: &str = "pinza";

/// The shipped block, its back, and the node the back waistline starts at.
///
/// The back of his own trousers is where the dart belongs: its waist run is the
/// one that does not match the waistband, and a dart is what takes the
/// difference out.
fn block() -> (Doc, PieceKey, PointKey) {
    let doc = block::trousers();
    let piece = doc.piece_named(block::BACK).expect("the block draws one");
    let node = doc
        .shows_label(piece, "cintura_cb")
        .expect("the block names it");
    (doc, piece, node)
}

/// A wedge of three straight nodes, written to three places on the waistline.
fn wedge(piece: PieceKey, after: Option<PointKey>) -> DartWedge {
    DartWedge {
        piece,
        after,
        nodes: [
            WedgeNode::line(Identity::New, Point::at(51.0, 0.0)),
            WedgeNode::line(Identity::New, Point::at(52.0, 12.0)),
            WedgeNode::line(Identity::New, Point::at(53.0, 0.0)),
        ],
    }
}

/// The dart a fresh cut carries: only its fold is read, because every key it
/// names is a key the edit itself issues.
fn asked() -> Dart {
    Dart {
        apex: PointKey::new(0, 0),
        legs: (PointKey::new(0, 0), PointKey::new(0, 0)),
        seam: SeamKey::new(0, 0),
        fold: FoldDirection::TowardEnd,
    }
}

fn cut(piece: PieceKey, after: Option<PointKey>) -> Command {
    Command::AddDart {
        identity: Identity::New,
        dart: asked(),
        wedge: Box::new(wedge(piece, after)),
    }
}

fn contour(doc: &Doc, piece: PieceKey) -> Vec<ContourNode> {
    doc.pieces
        .get(piece)
        .expect("the key is live")
        .contour
        .clone()
}

fn only_dart(doc: &Doc) -> (DartKey, Dart) {
    let (key, held) = doc.darts.iter().next().expect("the cut wrote one");
    (key, *held)
}

/// A wedge is cut where it is told to go, and the seam it issues joins its two
/// legs through its apex.
#[test]
fn a_wedge_is_cut_after_the_node_it_follows_and_sewn_leg_to_leg() {
    let (mut doc, piece, node) = block();
    let applied = cut(piece, Some(node))
        .apply(&mut doc)
        .expect("the node is on the contour");
    assert_eq!(applied.touched, vec![piece]);
    assert_eq!(applied.class, ChangeClass::Topology);
    assert_eq!(cut(piece, Some(node)).class(), ChangeClass::Topology);

    let (key, held) = only_dart(&doc);
    let drawn = doc.pieces.get(piece).expect("the key is live");
    assert_eq!(drawn.contour.len(), 12, "nine nodes and the wedge's three");
    assert_eq!(drawn.node_index(held.legs.0), Some(1));
    assert_eq!(drawn.node_index(held.apex), Some(2));
    assert_eq!(drawn.node_index(held.legs.1), Some(3));
    assert_eq!(held.fold, FoldDirection::TowardEnd);

    let sewn = doc.seams.get(held.seam).expect("the cut issued one");
    assert_eq!(sewn.a, EdgeRange::between(piece, held.legs.0, held.apex));
    assert_eq!(sewn.b, EdgeRange::between(piece, held.apex, held.legs.1));
    assert_eq!(
        sewn.orientation,
        SeamOrientation::Opposed,
        "the two sides meet at the apex"
    );
    assert_eq!(doc.seams.len(), 3, "the block's two, and the dart's");
    assert_eq!(applied.inverse, Command::RemoveDart { dart: key });
}

/// A wedge that opens the contour lands at its head, and the inverse says so by
/// naming no node to follow.
#[test]
fn a_wedge_that_opens_the_contour_follows_nothing() {
    let (mut doc, piece, _) = block();
    let applied = cut(piece, None)
        .apply(&mut doc)
        .expect("a contour has a head");
    let (_, held) = only_dart(&doc);
    let drawn = doc.pieces.get(piece).expect("the key is live");
    assert_eq!(drawn.node_index(held.legs.0), Some(0));

    let back = applied.inverse.apply(&mut doc).expect("the dart is live");
    let Command::AddDart { wedge, .. } = back.inverse else {
        panic!("the inverse cuts the wedge again");
    };
    assert_eq!(wedge.after, None);
}

/// Closing the dart back up leaves the contour the contour it was, node for
/// node, and takes the wedge's points and the dart's seam out with it.
#[test]
fn a_dart_cut_and_closed_leaves_the_contour_it_found() {
    let (mut doc, piece, node) = block();
    let was = contour(&doc, piece);
    let seams = doc.seams.len();
    let applied = cut(piece, Some(node))
        .apply(&mut doc)
        .expect("the node is on the contour");
    let (_, held) = only_dart(&doc);
    let points = [held.legs.0, held.apex, held.legs.1];
    assert_ne!(contour(&doc, piece), was, "the wedge is in");

    let back = applied.inverse.apply(&mut doc).expect("the dart is live");
    assert_eq!(back.touched, vec![piece]);
    assert_eq!(back.class, ChangeClass::Topology);
    assert_eq!(contour(&doc, piece), was, "node for node");
    assert!(doc.darts.is_empty());
    assert_eq!(doc.seams.len(), seams, "the dart's seam went with it");
    for point in points {
        assert_eq!(doc.points.get(point), None, "the wedge's point is out");
    }
}

/// Every key the cut issued comes back the same on redo: the three nodes of the
/// wedge, the seam that closes it and the dart itself. A key that changed would
/// leave the history telling a story the document cannot replay.
#[test]
fn every_key_the_cut_issued_comes_back_the_same_on_redo() {
    let (mut doc, piece, node) = block();
    let mut history = History::new();
    history.begin(CUT);
    history
        .edit(&mut doc, cut(piece, Some(node)))
        .expect("the node is on the contour");
    history.end();
    let (key, held) = only_dart(&doc);
    let points = [held.legs.0, held.apex, held.legs.1];

    history.undo(&mut doc).expect("the dart is live");
    assert!(doc.darts.is_empty());
    for point in points {
        assert_eq!(doc.points.get(point), None, "the wedge's point is out");
    }

    history.redo(&mut doc).expect("every slot is open again");
    let (again, back) = only_dart(&doc);
    assert_eq!(again, key, "the dart's own key");
    assert_eq!(back.seam, held.seam, "the seam's");
    assert_eq!([back.legs.0, back.apex, back.legs.1], points, "the wedge's");
}

/// The file is the other round trip: a dart written, read and written again is
/// the same bytes.
///
/// An undo does not take the file back to the very bytes it was, and no edit
/// that creates anything does: the slots the cut opened stay counted, which is
/// what stops a reopened document from handing out a key a live seam still
/// holds. Everything the file says about the pattern comes back, and the count
/// of slots opened is the whole of what does not.
#[test]
fn a_dart_survives_a_save_and_an_undo_takes_back_all_but_the_slots() {
    let (mut doc, piece, node) = block();
    let before = doc.to_canonical_json();
    let mut history = History::new();
    history
        .edit(&mut doc, cut(piece, Some(node)))
        .expect("the node is on the contour");

    let written = doc.to_canonical_json();
    let read = Doc::from_json(&written).expect("what the writer wrote, the reader reads");
    assert_eq!(read, doc);
    assert_eq!(read.to_canonical_json(), written);
    assert!(written.contains("\"fold\": \"toward_end\""), "{written}");

    history.undo(&mut doc).expect("the dart is live");
    let after = doc.to_canonical_json();
    assert_eq!(after.lines().count(), before.lines().count());
    let moved: Vec<&str> = after
        .lines()
        .zip(before.lines())
        .filter(|(now, was)| now != was)
        .map(|(now, _)| now.trim())
        .collect();
    assert!(!moved.is_empty(), "the cut opened slots");
    assert!(
        moved.iter().all(|line| line.starts_with("\"issued\"")),
        "{moved:?}"
    );

    history.redo(&mut doc).expect("every slot is open again");
    assert_eq!(doc.to_canonical_json(), written, "byte for byte");
}

/// A dart cut into a piece that already carries one is a second dart, not a
/// refusal: a waistband takes two, and each is its own wedge and its own seam.
#[test]
fn a_piece_takes_a_second_dart_beside_the_first() {
    let (mut doc, piece, node) = block();
    cut(piece, Some(node))
        .apply(&mut doc)
        .expect("the node is on the contour");
    let second = DartWedge {
        nodes: [
            WedgeNode::line(Identity::New, Point::at(57.0, 0.0)),
            WedgeNode::line(Identity::New, Point::at(58.0, 12.0)),
            WedgeNode::line(Identity::New, Point::at(59.0, 0.0)),
        ],
        ..wedge(piece, Some(node))
    };
    Command::AddDart {
        identity: Identity::New,
        dart: asked(),
        wedge: Box::new(second),
    }
    .apply(&mut doc)
    .expect("the node is still on the contour");
    assert_eq!(doc.darts.len(), 2);
    assert_eq!(doc.seams.len(), 4, "the block's two, and one per dart");
    assert_eq!(contour(&doc, piece).len(), 15);
}
