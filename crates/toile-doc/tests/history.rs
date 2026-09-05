#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_doc::{Axis, Binding, Command, Doc, DocError, History, PieceKey, PointKey, block};

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

/// One drag of `point`, frame by frame, as the interface would emit it.
fn drag(history: &mut History, doc: &mut Doc, point: PointKey, frames: &[f64]) {
    history.begin(DRAG);
    for &x in frames {
        history
            .edit(doc, move_to(point, x))
            .expect("the point is live");
    }
    history.end();
}

#[test]
fn a_drag_gesture_is_one_undo_entry() {
    let mut doc = block::trouser_front();
    let waist = node(&doc, "cintura_lat");
    let mut history = History::new();
    drag(&mut history, &mut doc, waist, &[22.1, 22.4, 22.9, 23.5]);
    assert_eq!(history.depth(), 1);
    assert_eq!(source_of(&doc, waist, Axis::X), "23.5");
}

#[test]
fn undo_restores_the_formula_not_a_literal() {
    let mut doc = block::trouser_front();
    let waist = node(&doc, "cintura_lat");
    let before = source_of(&doc, waist, Axis::X);
    let mut history = History::new();
    drag(&mut history, &mut doc, waist, &[22.1, 23.5]);
    history.undo(&mut doc).expect("the point is live");
    assert_eq!(source_of(&doc, waist, Axis::X), before);
    assert!(!doc.points.get(waist).expect("live").x.is_literal());
}

#[test]
fn a_gesture_keeps_the_first_inverse() {
    let mut doc = block::trouser_front();
    let hip = node(&doc, "cadera_lat");
    let before = source_of(&doc, hip, Axis::X);
    let mut history = History::new();
    // Undo goes back to before the gesture, never to its penultimate frame.
    drag(&mut history, &mut doc, hip, &[1.0, 2.0, 3.0]);
    history.undo(&mut doc).expect("the point is live");
    assert_eq!(source_of(&doc, hip, Axis::X), before);
    assert_eq!(history.depth(), 0);
    assert_eq!(history.redo_depth(), 1);
}

#[test]
fn redo_replays_the_forward_commands() {
    let mut doc = block::trouser_front();
    let waist = node(&doc, "cintura_lat");
    let mut history = History::new();
    drag(&mut history, &mut doc, waist, &[22.1, 23.5]);
    history.undo(&mut doc).expect("the point is live");
    let touched = history.redo(&mut doc).expect("the point is live");
    assert_eq!(source_of(&doc, waist, Axis::X), "23.5");
    assert_eq!(touched, [front(&doc)]);
    assert_eq!((history.depth(), history.redo_depth()), (1, 0));
}

#[test]
fn an_empty_gesture_leaves_no_entry() {
    let mut doc = block::trouser_front();
    let mut history = History::new();
    history.begin(DRAG);
    history.end();
    assert_eq!(history.depth(), 0);
    assert_eq!(history.undo_label(), None);
    let before = doc.clone();
    assert_eq!(history.undo(&mut doc), Ok(Vec::new()));
    assert_eq!(doc, before);
}

#[test]
fn a_new_entry_clears_the_redo_stack() {
    let mut doc = block::trouser_front();
    let waist = node(&doc, "cintura_lat");
    let mut history = History::new();
    drag(&mut history, &mut doc, waist, &[22.1]);
    history.undo(&mut doc).expect("the point is live");
    assert_eq!(history.redo_depth(), 1);
    drag(&mut history, &mut doc, waist, &[30.0]);
    assert_eq!(history.redo_depth(), 0);
    assert_eq!(history.redo(&mut doc), Ok(Vec::new()));
    assert_eq!(source_of(&doc, waist, Axis::X), "30");
}

#[test]
fn an_edit_outside_a_gesture_is_an_entry_of_its_own() {
    let mut doc = block::trouser_front();
    let waist = node(&doc, "cintura_lat");
    let mut history = History::new();
    history
        .edit(&mut doc, move_to(waist, 22.1))
        .expect("the point is live");
    history
        .edit(&mut doc, move_to(waist, 23.5))
        .expect("the point is live");
    assert_eq!(history.depth(), 2);
    history.undo(&mut doc).expect("the point is live");
    assert_eq!(source_of(&doc, waist, Axis::X), "22.1");
}

#[test]
fn two_points_of_one_gesture_stay_two_commands() {
    let mut doc = block::trouser_front();
    let (waist, hip) = (node(&doc, "cintura_lat"), node(&doc, "cadera_lat"));
    let before = source_of(&doc, hip, Axis::X);
    let mut history = History::new();
    history.begin(DRAG);
    for frame in [1.0, 2.0] {
        history
            .edit(&mut doc, move_to(waist, frame))
            .expect("the point is live");
        history
            .edit(&mut doc, move_to(hip, frame))
            .expect("the point is live");
    }
    history.end();
    assert_eq!(history.depth(), 1);
    history.undo(&mut doc).expect("both points are live");
    assert_eq!(source_of(&doc, hip, Axis::X), before);
}

#[test]
fn the_status_bar_reads_the_name_of_the_gesture() {
    let mut doc = block::trouser_front();
    let waist = node(&doc, "cintura_lat");
    let mut history = History::new();
    history.begin(DRAG);
    assert_eq!(history.undo_label(), None);
    history
        .edit(&mut doc, move_to(waist, 22.1))
        .expect("the point is live");
    assert_eq!(history.undo_label(), Some(DRAG));
    assert_eq!(history.depth(), 1);
    history.undo(&mut doc).expect("the point is live");
    assert_eq!(history.undo_label(), None);
    assert_eq!(history.redo_label(), Some(DRAG));
}

#[test]
fn an_edit_that_fails_records_nothing() {
    let mut doc = block::trouser_front();
    let mut history = History::new();
    let bad = Command::SetMeasure {
        mannequin: doc.resolve_with,
        name: "envergadura".to_owned(),
        to: 1.0,
    };
    history.begin(DRAG);
    let refused = history.edit(&mut doc, bad);
    history.end();
    assert_eq!(
        refused,
        Err(DocError::UnknownMeasure("envergadura".to_owned()))
    );
    assert_eq!(history.depth(), 0);
}

#[test]
fn an_undo_that_fails_puts_back_what_it_had_undone() {
    let mut doc = block::trouser_front();
    let (waist, hip) = (node(&doc, "cintura_lat"), node(&doc, "cadera_lat"));
    let mut history = History::new();
    history.begin(DRAG);
    for point in [waist, hip] {
        history
            .edit(&mut doc, move_to(point, 5.0))
            .expect("the point is live");
    }
    history.end();
    doc.points.remove(waist).expect("the key is live");
    assert!(history.undo(&mut doc).is_err());
    // The half of the undo that went through is rolled back, and the entry
    // stays: the document and the stack never disagree.
    assert_eq!(source_of(&doc, hip, Axis::X), "5");
    assert_eq!(history.depth(), 1);
}

#[test]
fn undo_names_the_pieces_the_edit_changed() {
    let mut doc = block::trouser_front();
    let waist = node(&doc, "cintura_lat");
    let mut history = History::new();
    drag(&mut history, &mut doc, waist, &[22.1]);
    assert_eq!(history.undo(&mut doc), Ok(vec![front(&doc)]));
}

#[test]
fn an_open_gesture_is_undone_whole() {
    let mut doc = block::trouser_front();
    let waist = node(&doc, "cintura_lat");
    let before = source_of(&doc, waist, Axis::X);
    let mut history = History::new();
    history.begin(DRAG);
    history
        .edit(&mut doc, move_to(waist, 22.1))
        .expect("the point is live");
    history.undo(&mut doc).expect("the point is live");
    assert_eq!(source_of(&doc, waist, Axis::X), before);
    assert_eq!(history.depth(), 0);
}
