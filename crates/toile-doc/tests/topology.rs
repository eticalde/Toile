#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_doc::{
    Command, Doc, DocError, EdgeRange, History, Identity, MeasureSet, Piece, PieceKey, Point,
    PointKey, Seam, SeamKey, SeamOrientation, SegmentEdit, Winding, block,
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

#[test]
fn a_piece_taken_off_the_table_comes_back_under_its_own_key() {
    let mut doc = block::trouser_front();
    let mut history = History::new();
    let piece = front(&doc);
    let contour = doc.pieces.get(piece).expect("the key is live").clone();
    let points = doc.points.len();

    history
        .edit(&mut doc, Command::RemovePiece { piece })
        .expect("the key is live");
    assert!(doc.pieces.is_empty());
    assert_eq!(doc.points.len(), points, "its points are the document's");

    history.undo(&mut doc).expect("the slot is free again");
    assert_eq!(doc.pieces.get(piece), Some(&contour));
    assert_eq!(front(&doc), piece);
}

#[test]
fn a_piece_named_after_another_is_refused_and_a_restored_one_is_not() {
    let mut doc = block::trouser_front();
    let piece = front(&doc);
    let knee = node(&doc, "rodilla_lat");
    let twin = Piece::polygon(block::FRONT, [knee], Winding::Cw);
    assert_eq!(
        Command::AddPiece {
            identity: Identity::New,
            piece: twin.clone(),
        }
        .apply(&mut doc),
        Err(DocError::DuplicatePieceName(block::FRONT.to_owned()))
    );

    // One gesture, two edits: the name is free by the time the second runs,
    // and undoing the pair leaves the front exactly where it started.
    let mut history = History::new();
    history.begin("replace the front");
    history
        .edit(&mut doc, Command::RemovePiece { piece })
        .expect("the key is live");
    history
        .edit(
            &mut doc,
            Command::AddPiece {
                identity: Identity::New,
                piece: twin,
            },
        )
        .expect("the name is free now");
    history.end();
    history.undo(&mut doc).expect("both edits come back out");
    assert_eq!(front(&doc), piece);
    assert_eq!(doc.pieces.len(), 1);
}

#[test]
fn a_piece_drawn_with_a_dead_point_is_refused() {
    let mut doc = block::trouser_front();
    let stray = doc.points.insert(Point::at(9.0, 9.0));
    doc.points.remove(stray).expect("the key is live");
    assert_eq!(
        Command::AddPiece {
            identity: Identity::New,
            piece: Piece::polygon("Vista", [stray], Winding::Cw),
        }
        .apply(&mut doc),
        Err(DocError::stale(stray))
    );
}

/// A point the same piece cites at two seats cannot be taken out from under
/// the surviving one: the removal is refused with the piece named, the same
/// treatment a citation from another piece gets.
#[test]
fn removing_a_node_the_same_piece_still_cites_is_refused() {
    let mut doc = Doc::new(MeasureSet::default());
    let a = doc.points.insert(Point::at(0.0, 0.0));
    let b = doc.points.insert(Point::at(10.0, 0.0));
    let c = doc.points.insert(Point::at(5.0, 8.0));
    let piece = doc
        .pieces
        .insert(Piece::polygon("Bolsillo", [a, b, a, c], Winding::Cw));
    let refused = Command::RemoveNode { piece, node: a }.apply(&mut doc).err();
    assert_eq!(refused, Some(DocError::Shared("Bolsillo".to_owned())));
    assert!(doc.points.get(a).is_some(), "the point is still alive");
    assert_eq!(anchors(&doc, piece).len(), 4, "and the contour untouched");
}
