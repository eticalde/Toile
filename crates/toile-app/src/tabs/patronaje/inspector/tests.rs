use toile_engine::draft::{Binding, Doc, MeasureSet, block};

use super::*;

/// The node the block binds on both axes, and what its Y is worth before
/// anything is broken.
fn cadera(doc: &Doc) -> (PieceKey, PointKey, f64) {
    let piece = doc
        .piece_named(block::FRONT)
        .expect("the block draws one piece");
    let point = doc
        .shows_label(piece, "cadera_lat")
        .expect("the block names the hip");
    let draft = Draft::from_doc(doc.clone()).expect("the block resolves");
    let cm = draft.resolved(point).expect("the hip resolves");
    (piece, point, cm[1])
}

/// A coordinate the document cannot resolve names its own axis, and leaves the
/// other one alone.
///
/// A point is abandoned at the first binding that fails, so the axis that never
/// got its turn has no number either. Reading that as a fault on both rows puts
/// an alert under a formula that is correct, and the person goes and edits it.
#[test]
fn a_broken_x_does_not_make_the_y_row_a_fault() {
    let mut doc = block::trouser_front();
    let (piece, point, y_cm) = cadera(&doc);
    doc.points.get_mut(point).expect("the key is live").x =
        Binding::parse("medida_que_no_existe").expect("the source parses");
    let draft = Draft::from_doc(doc).expect("a bad coordinate is a defect, not a refusal");

    let (note_x, fault_x) = coordinate(&draft, (piece, point), Axis::X);
    assert!(fault_x, "the broken axis is the one that faults");
    assert_eq!(note_x, "nombre desconocido: medida_que_no_existe");

    let (note_y, fault_y) = coordinate(&draft, (piece, point), Axis::Y);
    assert!(!fault_y, "the Y formula evaluates: {note_y}");
    assert_eq!(note_y, format!("= {y_cm:.1} cm"));
}

/// A point that resolves says what each axis comes to, as it always did.
#[test]
fn a_point_that_resolves_reports_both_of_its_coordinates() {
    let doc = block::trouser_front();
    let (piece, point, y_cm) = cadera(&doc);
    let draft = Draft::from_doc(doc).expect("the block resolves");
    let x_cm = draft.resolved(point).expect("the hip resolves")[0];

    assert_eq!(
        coordinate(&draft, (piece, point), Axis::X),
        (format!("= {x_cm:.1} cm"), false)
    );
    assert_eq!(
        coordinate(&draft, (piece, point), Axis::Y),
        (format!("= {y_cm:.1} cm"), false)
    );
}

/// A document with one body has no next body to step to.
///
/// Every product drawn from scratch is that document — a new one carries
/// exactly one mannequin — so the box that steps through them has to know the
/// difference and stop offering a press it cannot answer. It looked like a
/// working control only on the shipped block, which happens to carry two.
#[test]
fn a_document_with_one_body_has_no_next_one() {
    let fresh = Doc::new(MeasureSet::default());
    assert_eq!(fresh.mannequins.len(), 1);
    assert!(tape::next_body(&fresh).is_none());

    let doc = block::trouser_front();
    let next = tape::next_body(&doc).expect("the block carries two bodies");
    assert_ne!(next, doc.resolve_with, "a step lands on the other body");
}

mod desk;
mod elastic;
mod insert;
mod listing;
mod stacked;
