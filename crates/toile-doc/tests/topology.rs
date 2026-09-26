#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_doc::{
    Command, Doc, DocError, EdgeRange, History, Identity, Piece, PieceKey, Point, PointKey, Seam,
    SeamKey, SeamOrientation, SegmentEdit, Winding, block,
};

fn front(doc: &Doc) -> PieceKey {
    doc.piece_named(block::FRONT).expect("the block draws one")
}

fn node(doc: &Doc, label: &str) -> PointKey {
    doc.shows_label(front(doc), label)
        .unwrap_or_else(|| panic!("the block names {label}"))
}

/// The points a piece's contour passes through, in contour order.
fn anchors(doc: &Doc, piece: PieceKey) -> Vec<PointKey> {
    doc.pieces
        .get(piece)
        .expect("the key is live")
        .anchors()
        .collect()
}

/// A seam joining the hip stretch of the front to itself, and its two ends.
fn seam(doc: &mut Doc) -> (SeamKey, PointKey, PointKey) {
    let piece = front(doc);
    let (hip, knee) = (node(doc, "cadera_lat"), node(doc, "rodilla_lat"));
    let range = EdgeRange::between(piece, hip, knee);
    let key = doc
        .seams
        .insert(Seam::plain(range, range, SeamOrientation::Opposed));
    (key, hip, knee)
}

#[test]
fn undo_of_a_delete_restores_the_same_point_key() {
    let mut doc = block::trouser_front();
    let mut history = History::new();
    let piece = front(&doc);
    let knee = node(&doc, "rodilla_lat");
    let before = anchors(&doc, piece);

    history
        .edit(&mut doc, Command::RemoveNode { piece, node: knee })
        .expect("the contour runs through the knee");
    assert!(doc.points.get(knee).is_none());
    assert_eq!(anchors(&doc, piece).len(), before.len() - 1);

    let touched = history.undo(&mut doc).expect("the slot is free again");
    assert_eq!(touched, [piece]);
    assert_eq!(
        anchors(&doc, piece),
        before,
        "the key and its seat come back"
    );
    assert_eq!(
        doc.label_of(piece, knee).as_deref(),
        Some("rodilla_lat"),
        "and the point it names is the one that was taken away"
    );
}

#[test]
fn a_seam_referencing_a_deleted_node_survives_the_undo_cycle() {
    let mut doc = block::trouser_front();
    let mut history = History::new();
    let piece = front(&doc);
    let (key, hip, knee) = seam(&mut doc);

    history
        .edit(&mut doc, Command::RemoveNode { piece, node: hip })
        .expect("the contour runs through the hip");
    // The seam still names the key. The reference is not dangling for good:
    // the arena never recycles the slot, so nothing else can take it while the
    // deletion sits on the undo stack.
    assert_eq!(doc.seams.get(key).map(|held| held.a.head.from), Some(hip));
    assert!(doc.points.get(hip).is_none());

    history.undo(&mut doc).expect("the slot is free again");
    let held = doc.seams.get(key).expect("the seam was never touched");
    assert_eq!(held.a.head.from, hip);
    assert_eq!(held.a.tail.from, knee);
    assert!(doc.points.get(hip).is_some());
    assert_eq!(doc.label_of(piece, hip).as_deref(), Some("cadera_lat"));
}

#[test]
fn deleting_a_node_takes_its_handles_and_undo_gives_them_back() {
    let mut doc = block::trouser_front();
    let mut history = History::new();
    let piece = front(&doc);
    let waist = node(&doc, "cintura_lat");
    let (out, into) = doc
        .pieces
        .get(piece)
        .expect("the key is live")
        .contour
        .iter()
        .find(|held| held.point == waist)
        .and_then(|held| held.segment.handles())
        .expect("the block bends the tract leaving the waist");
    let count = doc.points.len();

    history
        .edit(&mut doc, Command::RemoveNode { piece, node: waist })
        .expect("the contour runs through the waist");
    assert_eq!(doc.points.len(), count - 3, "the node and its two handles");

    history.undo(&mut doc).expect("the slots are free again");
    assert_eq!(doc.points.len(), count);
    assert_eq!(
        doc.label_of(piece, out).as_deref(),
        None,
        "a handle is no node"
    );
    assert_eq!(
        doc.points.get(into).map(|held| held.label.clone()),
        Some(Some("manija_cadera_2".to_owned())),
        "the handle comes back with the name it had grown"
    );
}

#[test]
fn a_node_another_piece_draws_itself_with_is_not_deleted_silently() {
    let mut doc = block::trouser_front();
    let piece = front(&doc);
    let knee = node(&doc, "rodilla_lat");
    let shared = [knee, node(&doc, "bajo_lat"), node(&doc, "bajo_int")];
    doc.pieces
        .insert(Piece::polygon("Vista", shared, Winding::Cw));

    let refused = Command::RemoveNode { piece, node: knee }.apply(&mut doc);
    assert_eq!(refused, Err(DocError::Shared("Vista".to_owned())));
    assert!(doc.points.get(knee).is_some());
}

#[test]
fn a_node_the_contour_does_not_run_through_is_an_error_not_a_panic() {
    let mut doc = block::trouser_front();
    let piece = front(&doc);
    let stray = doc.points.insert(Point::at(9.0, 9.0));
    assert_eq!(
        Command::RemoveNode { piece, node: stray }.apply(&mut doc),
        Err(DocError::NoSuchNode)
    );
    assert_eq!(
        Command::InsertNode {
            piece,
            after: Some(stray),
            identity: Identity::New,
            value: Point::at(1.0, 1.0),
            segment: SegmentEdit::Line,
            samples: 1,
        }
        .apply(&mut doc),
        Err(DocError::NoSuchNode)
    );
}

#[test]
fn a_node_inserted_at_the_head_opens_the_contour() {
    let mut doc = block::trouser_front();
    let piece = front(&doc);
    let before = anchors(&doc, piece);
    let applied = Command::InsertNode {
        piece,
        after: None,
        identity: Identity::New,
        value: Point::at(-1.0, -1.0),
        segment: SegmentEdit::Line,
        samples: 1,
    }
    .apply(&mut doc)
    .expect("a contour always has a head");
    let after = anchors(&doc, piece);
    assert_eq!(after[1..], before[..]);
    applied.inverse.apply(&mut doc).expect("the node is there");
    assert_eq!(anchors(&doc, piece), before);
}

#[test]
fn a_curve_inserted_at_one_sample_is_refused_before_anything_moves() {
    let mut doc = block::trouser_front();
    let piece = front(&doc);
    let waist = node(&doc, "cintura_lat");
    let count = doc.points.len();
    let refused = Command::InsertNode {
        piece,
        after: Some(waist),
        identity: Identity::New,
        value: Point::at(1.0, 1.0),
        segment: SegmentEdit::cubic(Point::at(2.0, 2.0), Point::at(3.0, 3.0)),
        samples: 1,
    }
    .apply(&mut doc);
    assert_eq!(refused, Err(DocError::sampling(1)));
    assert_eq!(doc.points.len(), count, "no handle was issued a key");
}
