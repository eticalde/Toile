#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_doc::{Axis, Binding, Command, Doc, History, PieceKey, PointKey, block};

const DRAG: &str = "mover punto";

fn front(doc: &Doc) -> PieceKey {
    doc.piece_named(block::FRONT).expect("the block draws one")
}

fn node(doc: &Doc, label: &str) -> PointKey {
    doc.shows_label(front(doc), label)
        .unwrap_or_else(|| panic!("the block names {label}"))
}

fn source_of(doc: &Doc, point: PointKey, axis: Axis) -> String {
    doc.points
        .get(point)
        .expect("the key is live")
        .binding(axis)
        .source()
        .into_owned()
}

fn move_to(point: PointKey, x: f64) -> Command {
    Command::MovePoint {
        point,
        to: [Binding::literal(x), Binding::literal(0.0)],
    }
}

/// Renaming inside one gesture is one transaction, and undo replays it as one.
///
/// Each step is legal on the way in, and the middle of the way back out is
/// not: the inverses put `cadera_lat` back on its own point while the other
/// one still holds it. A per-command check refuses that and strands the entry
/// on the stack, taking every entry under it out of reach with it.
#[test]
fn a_gesture_that_swapped_two_names_is_undone_whole() {
    let mut doc = block::trouser_front();
    let before = doc.clone();
    let (waist, hip) = (node(&doc, "cintura_lat"), node(&doc, "cadera_lat"));
    let mut history = History::new();
    history.begin("renombrar");
    for (point, to) in [
        (waist, "libre"),
        (hip, "cintura_lat"),
        (waist, "cadera_lat"),
    ] {
        history
            .edit(
                &mut doc,
                Command::LabelPoint {
                    point,
                    to: Some(to.to_owned()),
                },
            )
            .expect("each step is free when it is made");
    }
    history.end();
    assert_eq!(history.undo(&mut doc), Ok(vec![front(&doc)]));
    assert_eq!(doc, before);
}

/// The same shape, on piece names.
#[test]
fn a_gesture_that_swapped_two_piece_names_is_undone_whole() {
    let mut doc = block::trouser_front();
    let a = front(&doc);
    let mut second = doc.pieces.get(a).expect("the block draws one").clone();
    second.name = "Trasero".to_owned();
    let b = doc.pieces.insert(second);
    let before = doc.clone();
    let mut history = History::new();
    history.begin("renombrar");
    for (piece, to) in [(a, "libre"), (b, block::FRONT), (a, "Trasero")] {
        history
            .edit(
                &mut doc,
                Command::RenamePiece {
                    piece,
                    to: to.to_owned(),
                },
            )
            .expect("each step is free when it is made");
    }
    history.end();
    history.undo(&mut doc).expect("an entry is one transaction");
    assert_eq!(doc, before);
}

#[test]
fn a_cancelled_gesture_leaves_nothing_for_redo() {
    let mut doc = block::trouser_front();
    let waist = node(&doc, "cintura_lat");
    let before = source_of(&doc, waist, Axis::X);
    let mut history = History::new();
    history.begin(DRAG);
    for x in [22.1, 23.5, 25.0] {
        history
            .edit(&mut doc, move_to(waist, x))
            .expect("the point is live");
    }
    assert_eq!(history.cancel(&mut doc), Ok(vec![front(&doc)]));
    assert_eq!(source_of(&doc, waist, Axis::X), before);
    assert_eq!((history.depth(), history.redo_depth()), (0, 0));
    assert_eq!(history.redo_label(), None);
    assert_eq!(history.redo(&mut doc), Ok(Vec::new()));
    assert_eq!(source_of(&doc, waist, Axis::X), before);
}

#[test]
fn cancelling_with_nothing_in_hand_touches_nothing() {
    let mut doc = block::trouser_front();
    let before = doc.clone();
    let mut history = History::new();
    assert_eq!(history.cancel(&mut doc), Ok(Vec::new()));
    assert_eq!(doc, before);
}
