#![allow(missing_docs, reason = "a test crate publishes no API surface")]
#![allow(
    clippy::float_cmp,
    reason = "a length nobody can point at is exactly zero"
)]

use toile_engine::draft::{
    Axis, Binding, Command, Defect, Draft, DraftError, EnvError, PieceKey, PointKey, Recompile,
    block,
};

fn front() -> (Draft, PieceKey) {
    let draft = Draft::from_doc(block::trouser_front()).expect("the block resolves");
    let piece = draft
        .doc()
        .piece_named("Delantero")
        .expect("the block draws one piece");
    (draft, piece)
}

#[test]
fn the_block_resolves_to_nine_nodes_in_both_units() {
    let (draft, piece) = front();
    assert_eq!(draft.points_cm(piece).len(), 9);
    assert_eq!(draft.outline(piece).len(), 9);
    assert!(draft.defects(piece).is_empty());
    assert_eq!(draft.topology(piece), 0);
}

#[test]
fn the_side_seam_measures_what_the_status_bar_says() {
    let (draft, piece) = front();
    let nodes = draft.points_cm(piece);
    let (waist, hem) = (nodes[1].0, nodes[4].0);
    assert!((draft.run_length_cm(piece, waist, hem) - 104.48).abs() < 0.01);
    assert!((draft.perimeter_cm(piece) - 255.21).abs() < 0.01);
}

#[test]
fn a_run_between_nodes_of_another_piece_measures_nothing() {
    let (draft, piece) = front();
    let stray = PointKey::new(99, 0);
    assert_eq!(draft.run_length_cm(piece, stray, stray), 0.0);
    assert_eq!(draft.run_length_cm(PieceKey::new(9, 0), stray, stray), 0.0);
}

#[test]
fn changing_the_body_reshapes_the_piece_without_remeshing_it() {
    let (mut draft, piece) = front();
    let before = draft.outline(piece).to_vec();
    let other = draft
        .doc()
        .mannequin_named("Talla 42")
        .expect("the block carries a second body");
    let what = draft
        .edit(Command::ResolveWith { mannequin: other })
        .expect("the second body is live");
    assert_eq!(what, Recompile::Shape(vec![piece]));
    assert_eq!(draft.outline(piece).len(), before.len());
    assert_ne!(draft.outline(piece), before.as_slice());
    assert_eq!(draft.topology(piece), 0);
}

#[test]
fn an_edit_that_breaks_the_formulas_is_taken_back_out() {
    let (mut draft, piece) = front();
    let before = draft.outline(piece).to_vec();
    let raya = draft
        .doc()
        .variable_named("raya")
        .expect("the block declares it");
    let broken = draft.edit(Command::SetVariable {
        variable: raya,
        to: Binding::parse("raya + 1").expect("the source parses"),
    });
    assert!(matches!(broken, Err(DraftError::Env(EnvError::Order(_)))));
    assert_eq!(draft.outline(piece), before.as_slice());
    assert!(draft.env().value("raya").is_some());
}

#[test]
fn a_piece_that_stops_resolving_keeps_its_last_good_drawing() {
    let (mut draft, piece) = front();
    let before = draft.outline(piece).to_vec();
    let node = draft.points_cm(piece)[3].0;
    draft
        .edit(Command::SetBinding {
            point: node,
            axis: Axis::X,
            to: Binding::parse("largo_del_brazo").expect("the source parses"),
        })
        .expect("the document takes the binding");
    assert_eq!(draft.outline(piece), before.as_slice());
    assert!(matches!(draft.defects(piece), [Defect::Binding { .. }]));
    assert_eq!(draft.resolved(node), None);
}

#[test]
fn a_document_whose_names_do_not_resolve_is_refused_at_the_door() {
    let mut doc = block::trouser_front();
    let raya = doc.variable_named("raya").expect("the block declares it");
    doc.variables.get_mut(raya).expect("the key is live").value =
        Binding::parse("cadera").expect("the source parses");
    doc.mannequins
        .remove(doc.resolve_with)
        .expect("the block carries the body it resolves against");
    assert_eq!(
        Draft::from_doc(doc),
        Err(DraftError::Env(EnvError::NoMannequin))
    );
}
