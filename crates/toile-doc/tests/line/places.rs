use toile_doc::{
    Command, Doc, DocError, History, Identity, LineEdit, LineKey, LineKind, Point, PointKey,
    VertexEdit, block,
};

use super::{drawn, front, node};

/// A buttonhole of two places of its own, which is what a buttonhole is:
/// nothing on the contour says where one goes.
fn hole(doc: &Doc) -> LineEdit {
    LineEdit::new(
        front(doc),
        LineKind::Buttonhole,
        VertexEdit::free(Point::at(3.0, 4.0)),
    )
    .to(VertexEdit::free(Point::at(3.0, 6.0)))
    .named("ojal 1")
}

fn draw(doc: &mut Doc, edit: LineEdit) -> LineKey {
    let applied = Command::AddLine {
        identity: Identity::New,
        line: Box::new(edit),
    }
    .apply(doc)
    .expect("a piece takes a line of its own places");
    match applied.inverse {
        Command::RemoveLine { line } => line,
        other => panic!("the inverse of drawing a line is rubbing it out: {other:?}"),
    }
}

/// A place off the contour is a point of the document, so the drawing is what
/// creates it and rubbing the line out is what takes it away — the very
/// contract a curve's handles already answer to. The undo gives the same keys
/// back with the bindings they had, so the bytes come back too.
#[test]
fn a_place_of_its_own_arrives_and_leaves_with_the_line() {
    let mut doc = block::trouser_front();
    let points = doc.points.len();
    let edit = hole(&doc);
    let key = draw(&mut doc, edit);
    assert_eq!(doc.points.len(), points + 2, "both places landed");
    assert_eq!(drawn(&doc, key).label.as_deref(), Some("ojal 1"));
    let before = doc.clone();

    let mut history = History::new();
    history
        .edit(&mut doc, Command::RemoveLine { line: key })
        .expect("the line is live");
    assert_eq!(doc.points.len(), points, "and both went with the line");
    history.undo(&mut doc).expect("every slot is free again");
    assert_eq!(doc, before, "the same keys, the same bindings");
}

/// A point the contour draws itself with outlives any line that names it: the
/// inverse cites it rather than carrying it, and it stays where it is.
#[test]
fn a_place_the_contour_shares_stays_when_the_line_goes() {
    let mut doc = block::trouser_front();
    let piece = front(&doc);
    let corner = node(&doc, "cintura_cf");
    let edit = LineEdit::new(piece, LineKind::Reference, VertexEdit::Cited(corner))
        .to(VertexEdit::free(Point::at(3.0, 6.0)));
    let points = doc.points.len();
    let key = draw(&mut doc, edit);
    assert_eq!(doc.points.len(), points + 1, "only the free place landed");

    let applied = Command::RemoveLine { line: key }
        .apply(&mut doc)
        .expect("the line is live");
    assert_eq!(doc.points.len(), points, "the node the contour draws stays");
    let Command::AddLine { line, .. } = &applied.inverse else {
        panic!("the inverse draws the line again: {:?}", applied.inverse);
    };
    assert_eq!(line.head, VertexEdit::Cited(corner));
}

/// A place two lines name is shared as well, so the first removal leaves it and
/// the second takes it: what decides is whether anything else still needs it.
#[test]
fn a_place_two_lines_name_stays_for_the_second_of_them() {
    let mut doc = block::trouser_front();
    let piece = front(&doc);
    let mark = doc.points.insert(Point::at(2.0, 2.0));
    let keys: Vec<LineKey> = [5.0, 9.0]
        .into_iter()
        .map(|to| {
            let edit = LineEdit::new(piece, LineKind::Placement, VertexEdit::Cited(mark))
                .to(VertexEdit::free(Point::at(2.0, to)));
            draw(&mut doc, edit)
        })
        .collect();

    Command::RemoveLine { line: keys[0] }
        .apply(&mut doc)
        .expect("the line is live");
    assert!(doc.points.get(mark).is_some(), "the other line needs it");
    Command::RemoveLine { line: keys[1] }
        .apply(&mut doc)
        .expect("the line is live");
    assert!(doc.points.get(mark).is_none(), "and now nothing does");
}

/// A point one line names twice goes with the line: the run is the only thing
/// naming it, so taking it away leaves nothing drawing with a key that leads
/// nowhere.
///
/// That is how a run which closes is written — it comes back to the place it
/// started from — so leaving such a point behind grows the arena one orphan
/// every time one of those runs is rubbed out.
#[test]
fn a_place_one_line_names_twice_goes_with_the_line() {
    let mut doc = block::trouser_front();
    let piece = front(&doc);
    let corner = doc.points.insert(Point::at(2.0, 2.0));
    let points = doc.points.len();
    let edit = LineEdit::new(piece, LineKind::Placement, VertexEdit::Cited(corner))
        .to(VertexEdit::free(Point::at(6.0, 2.0)))
        .to(VertexEdit::free(Point::at(6.0, 6.0)))
        .to(VertexEdit::Cited(corner));
    let key = draw(&mut doc, edit);
    assert_eq!(doc.points.len(), points + 2, "the two new places landed");
    let before = doc.clone();

    let applied = Command::RemoveLine { line: key }
        .apply(&mut doc)
        .expect("the line is live");
    assert_eq!(doc.points.len(), points - 1, "the closing place went too");
    assert!(doc.points.get(corner).is_none(), "and left no orphan");
    let Command::AddLine { line, .. } = &applied.inverse else {
        panic!("the inverse draws the line again: {:?}", applied.inverse);
    };
    let restored = matches!(
        &line.head,
        VertexEdit::Free { identity: Identity::Restored(back), .. } if *back == corner
    );
    assert!(restored, "the first place carries it back: {:?}", line.head);
    assert_eq!(
        line.spans.last().map(|span| &span.to),
        Some(&VertexEdit::Cited(corner)),
        "and the last cites what the first restores"
    );
    applied.inverse.apply(&mut doc).expect("the slots are free");
    assert_eq!(doc, before, "the same keys, the same bindings");
}

/// Two places asking for one key is a plan no arena can arrange, and it is
/// refused before either is written.
#[test]
fn two_places_that_want_one_key_are_refused_before_anything_moves() {
    let mut doc = block::trouser_front();
    let piece = front(&doc);
    let key = doc.points.insert(Point::at(0.0, 0.0));
    doc.points.remove(key).expect("the key is live");
    let edit = LineEdit::new(
        piece,
        LineKind::Slit,
        VertexEdit::restored(key, Point::at(1.0, 1.0)),
    )
    .to(VertexEdit::restored(key, Point::at(2.0, 2.0)));
    let command = Command::AddLine {
        identity: Identity::New,
        line: Box::new(edit),
    };
    assert_eq!(command.apply(&mut doc), Err(DocError::occupied(key)));
    assert!(doc.lines.is_empty(), "nothing landed");
    assert!(
        doc.points.get(key).is_none(),
        "the freed slot is still free"
    );
}

/// A place that asks for a key another point still holds is refused as well,
/// and so is one that names a slot the arena never issued.
#[test]
fn a_place_that_asks_for_a_taken_key_is_refused() {
    let mut doc = block::trouser_front();
    let piece = front(&doc);
    let taken = node(&doc, "cintura_cf");
    let stray = PointKey::new(90, 0);
    for (place, expected) in [
        (
            VertexEdit::restored(taken, Point::at(1.0, 1.0)),
            DocError::occupied(taken),
        ),
        (
            VertexEdit::restored(stray, Point::at(1.0, 1.0)),
            DocError::stale(stray),
        ),
    ] {
        let edit = LineEdit::new(piece, LineKind::Fold, place).to(VertexEdit::Cited(taken));
        let command = Command::AddLine {
            identity: Identity::New,
            line: Box::new(edit),
        };
        assert_eq!(command.apply(&mut doc), Err(expected));
    }
    assert!(doc.lines.is_empty(), "nothing landed");
}
