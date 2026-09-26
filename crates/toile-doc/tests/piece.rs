#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_doc::{
    Command, Doc, DocError, History, Identity, MeasureSet, Piece, PieceKey, Point, PointKey,
    Winding, block,
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

/// A piece nothing else names comes off; one that is still drawn on does not.
///
/// The loader refuses a seam, an elastic, a hang, a line, a notch or an axis
/// that names a piece the file does not carry, and the editor has to refuse
/// what the loader refuses or a person saves a pattern that never opens again.
/// Measured before this guard: a two-piece block, the trash icon on one piece,
/// eight hundred milliseconds of autosave, and the file on disk was gone — the
/// undo that could have repaired it dies with the process.
#[test]
fn a_piece_something_is_still_drawn_on_does_not_come_off_the_table() {
    let doc = block::trousers();
    let front = doc.piece_named(block::FRONT).expect("the block draws one");
    let back = doc.piece_named(block::BACK).expect("and another");

    // The shipped block's two pieces are sewn to each other, so each of them
    // is named by something from the moment it is opened.
    for piece in [front, back] {
        let mut doc = doc.clone();
        let before = doc.to_canonical_json();
        let why = Command::RemovePiece { piece }
            .apply(&mut doc)
            .expect_err("something is drawn on it");
        assert_eq!(why, DocError::PieceStillDrawn);
        assert_eq!(doc.to_canonical_json(), before, "and nothing was written");
    }

    // And a piece nothing names still comes off, and what it writes reopens.
    let mut doc = Doc::new(MeasureSet::new("Maniquí", []));
    let corners: Vec<PointKey> = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]]
        .iter()
        .map(|&[x, y]| doc.points.insert(Point::at(x, y)))
        .collect();
    let lone = doc
        .pieces
        .insert(Piece::polygon("Sola", corners, Winding::Cw));
    Command::RemovePiece { piece: lone }
        .apply(&mut doc)
        .expect("nothing is drawn on it");
    let written = doc.to_canonical_json();
    assert_eq!(
        Doc::from_json(&written).as_ref(),
        Ok(&doc),
        "and the file it writes opens again"
    );
}
