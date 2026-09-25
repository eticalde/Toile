#![allow(missing_docs, reason = "a test crate publishes no API surface")]

mod edits;
mod places;

use toile_doc::{
    ChangeClass, Command, Doc, DocError, EdgeAnchor, History, Identity, InternalLine, LineEdit,
    LineKey, LineKind, PieceKey, Point, PointKey, SAMPLES, SegmentEdit, VertexEdit, block,
};

pub(crate) fn front(doc: &Doc) -> PieceKey {
    doc.piece_named(block::FRONT).expect("the block draws one")
}

pub(crate) fn node(doc: &Doc, label: &str) -> PointKey {
    doc.shows_label(front(doc), label)
        .unwrap_or_else(|| panic!("the block names {label}"))
}

pub(crate) fn on(doc: &Doc, label: &str, t: f64) -> VertexEdit {
    VertexEdit::Contour(EdgeAnchor {
        piece: front(doc),
        from: node(doc, label),
        t,
    })
}

/// A place of its own on a point the document already carries, the way an
/// import writes one: the construction names it, so the line only cites it.
pub(crate) fn loose(doc: &mut Doc, x: f64, y: f64) -> VertexEdit {
    VertexEdit::Cited(doc.points.insert(Point::at(x, y)))
}

/// A pocket mouth across the front: one end on the waist, one loose.
pub(crate) fn mouth(doc: &mut Doc) -> LineEdit {
    let head = on(doc, "cintura_lat", 0.25);
    let tail = loose(doc, 8.0, 14.0);
    LineEdit::new(front(doc), LineKind::Slit, head)
        .to(tail)
        .named("ranura pill")
}

pub(crate) fn draw(doc: &mut Doc, edit: LineEdit) -> LineKey {
    let applied = Command::AddLine {
        identity: Identity::New,
        line: Box::new(edit),
    }
    .apply(doc)
    .expect("every place is one the front carries");
    match applied.inverse {
        Command::RemoveLine { line } => line,
        other => panic!("the inverse of drawing a line is rubbing it out: {other:?}"),
    }
}

pub(crate) fn drawn(doc: &Doc, key: LineKey) -> InternalLine {
    doc.lines.get(key).expect("the line is live").clone()
}

/// Nothing is derived from an internal line, so nothing is re-derived when one
/// appears: the same price as moving a piece on the product overview.
#[test]
fn drawing_a_line_costs_the_derivation_nothing() {
    let mut doc = block::trouser_front();
    let edit = mouth(&mut doc);
    let command = Command::AddLine {
        identity: Identity::New,
        line: Box::new(edit),
    };
    assert_eq!(command.class(), ChangeClass::Metadata);
    let applied = command
        .apply(&mut doc)
        .expect("both places are on the front");
    assert_eq!(applied.class, ChangeClass::Metadata);
    assert!(applied.touched.is_empty());
    assert_eq!(doc.lines.len(), 1);
}

#[test]
fn drawing_a_line_and_rubbing_it_out_restores_the_same_key() {
    let mut doc = block::trouser_front();
    let edit = mouth(&mut doc);
    let key = draw(&mut doc, edit);
    let line = drawn(&doc, key);

    let removed = Command::RemoveLine { line: key }
        .apply(&mut doc)
        .expect("the line is live");
    assert!(doc.lines.get(key).is_none());
    removed.inverse.apply(&mut doc).expect("the slot is free");
    assert_eq!(doc.lines.get(key), Some(&line), "the same key lives");
}

/// A line runs from one place to another. One place is a dot, and a dot is not
/// something a pattern can be cut by.
#[test]
fn a_line_through_fewer_than_two_places_is_refused() {
    let mut doc = block::trouser_front();
    let head = on(&doc, "cintura_cf", 0.0);
    let alone = LineEdit::new(front(&doc), LineKind::Placement, head);
    let command = Command::AddLine {
        identity: Identity::New,
        line: Box::new(alone),
    };
    assert_eq!(command.apply(&mut doc), Err(DocError::ShortLine));
    assert!(doc.lines.is_empty(), "nothing landed");
}

/// A place on a contour answers to one rule whoever writes it, so a line is
/// refused exactly where a seam side and an elastic are: on a handle, on a key
/// that leads nowhere, and on another piece's contour.
#[test]
fn a_line_is_anchored_by_the_rule_a_seam_side_answers_to() {
    let mut doc = block::trousers();
    let piece = front(&doc);
    let handle = doc
        .pieces
        .get(piece)
        .and_then(|held| held.contour.iter().find_map(|node| node.segment.handles()))
        .map(|(out, _)| out)
        .expect("the front bends two tracts");
    let stray = PointKey::new(90, 0);
    let back = doc.piece_named(block::BACK).expect("the block draws one");
    let head = on(&doc, "cintura_cf", 0.0);
    let anchored = |piece, from, t| VertexEdit::Contour(EdgeAnchor { piece, from, t });

    for (tail, expected) in [
        (anchored(piece, handle, 0.0), DocError::NoSuchNode),
        (anchored(piece, stray, 0.0), DocError::stale(stray)),
        (
            anchored(back, node(&doc, "cintura_lat"), 0.0),
            DocError::SplitInternalLine,
        ),
        (
            anchored(piece, node(&doc, "cintura_lat"), 1.5),
            DocError::AnchorFraction,
        ),
        (VertexEdit::Cited(stray), DocError::stale(stray)),
    ] {
        let command = Command::AddLine {
            identity: Identity::New,
            line: Box::new(LineEdit::new(piece, LineKind::Stitch, head.clone()).to(tail)),
        };
        assert_eq!(command.apply(&mut doc), Err(expected));
    }
    assert!(doc.lines.is_empty(), "nothing landed");
}

/// A buttonhole is placed by two free points and touches no contour, and it is
/// still cut out with one piece — so the line names it, and a piece the
/// document has lost is refused before the line lands.
#[test]
fn a_line_of_free_places_still_names_the_piece_it_is_cut_with() {
    let mut doc = block::trouser_front();
    let head = loose(&mut doc, 2.0, 3.0);
    let tail = loose(&mut doc, 2.0, 5.5);
    let piece = front(&doc);
    let hole = LineEdit::new(piece, LineKind::Buttonhole, head)
        .to(tail)
        .named("ojal 1");
    let key = draw(&mut doc, hole.clone());
    assert_eq!(drawn(&doc, key).piece, piece);
    assert!(drawn(&doc, key).kind.opens_the_cloth());

    let ghost = LineEdit {
        piece: PieceKey::new(9, 0),
        ..hole
    };
    let command = Command::AddLine {
        identity: Identity::New,
        line: Box::new(ghost),
    };
    assert_eq!(
        command.apply(&mut doc),
        Err(DocError::stale(PieceKey::new(9, 0)))
    );
}

/// A span that bends hangs on two handles that are points of the document, so
/// rubbing the line out takes them away and the undo gives back the very same
/// keys, with the bindings and names they had grown.
#[test]
fn the_handles_of_a_curved_span_come_back_under_their_own_keys() {
    let mut doc = block::trouser_front();
    let head = on(&doc, "cintura_cf", 0.0);
    let tail = loose(&mut doc, 6.0, 9.0);
    let curve = SegmentEdit::cubic(
        Point::at(2.0, 3.0).named("manija_ranura"),
        Point::at(4.0, 7.0),
    );
    let edit = LineEdit::new(front(&doc), LineKind::Slit, head).curving(tail, curve, 12);
    let points = doc.points.len();
    let key = draw(&mut doc, edit);
    assert_eq!(doc.points.len(), points + 2, "the two handles landed");
    let handles: Vec<PointKey> = drawn(&doc, key).handles().collect();
    let before = doc.clone();

    let mut history = History::new();
    history
        .edit(&mut doc, Command::RemoveLine { line: key })
        .expect("the line is live");
    // The two handles, and the loose place as well: nothing else in the
    // document named it, so it was the line's own by then.
    assert_eq!(doc.points.len(), points - 1, "the handles went with it");
    history.undo(&mut doc).expect("every slot is free again");
    assert_eq!(doc, before);
    assert_eq!(drawn(&doc, key).handles().collect::<Vec<_>>(), handles);
}

#[test]
fn a_span_that_bends_is_flattened_by_the_rule_a_tract_answers_to() {
    let mut doc = block::trouser_front();
    let head = on(&doc, "cintura_cf", 0.0);
    let tail = loose(&mut doc, 6.0, 9.0);
    let curve = SegmentEdit::cubic(Point::at(2.0, 3.0), Point::at(4.0, 7.0));
    for count in [0, 1, SAMPLES.1 + 1] {
        let edit = LineEdit::new(front(&doc), LineKind::Stitch, head.clone()).curving(
            tail.clone(),
            curve.clone(),
            count,
        );
        let command = Command::AddLine {
            identity: Identity::New,
            line: Box::new(edit),
        };
        assert_eq!(command.apply(&mut doc), Err(DocError::sampling(count)));
    }
    assert!(doc.lines.is_empty(), "nothing landed");
}
