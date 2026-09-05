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
    // Nine nodes, but forty-seven points on the line they draw: the hip is
    // flattened into twenty-four and the crotch into sixteen.
    assert_eq!(draft.outline(piece).len(), 47);
    assert!(draft.defects(piece).is_empty());
    assert_eq!(draft.topology(piece), 0);
}

#[test]
fn the_side_seam_measures_what_the_status_bar_says() {
    let (draft, piece) = front();
    let nodes = draft.points_cm(piece);
    let (waist, hem) = (nodes[1].0, nodes[4].0);
    assert!((draft.run_length_cm(piece, waist, hem) - 104.60).abs() < 0.01);
    assert!((draft.perimeter_cm(piece) - 256.16).abs() < 0.01);
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
    assert_eq!(draft.undo_depth(), 0, "a refused edit is not on the stack");
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

/// The source of one coordinate, the way the inspector reads it.
fn source(draft: &Draft, point: PointKey, axis: Axis) -> String {
    draft
        .doc()
        .points
        .get(point)
        .expect("the key is live")
        .binding(axis)
        .source()
        .into_owned()
}

#[test]
fn a_whole_drag_is_one_undo_entry() {
    let (mut draft, piece) = front();
    let node = draft.points_cm(piece)[1].0;
    let before = draft.resolved(node).expect("the node resolves");
    draft.begin_gesture("mover punto");
    for step in 1..=4 {
        let to = [
            Binding::literal(before[0] + f64::from(step)),
            Binding::literal(before[1]),
        ];
        draft
            .edit(Command::MovePoint { point: node, to })
            .expect("the document takes the move");
    }
    draft.end_gesture();
    assert_eq!(draft.undo_depth(), 1);
    assert_eq!(draft.undo_label(), Some("mover punto"));
    assert_eq!(draft.undo(), Ok(Recompile::Shape(vec![piece])));
    assert_eq!(draft.resolved(node), Some(before));
    assert_eq!(draft.undo_depth(), 0);
}

#[test]
fn undo_gives_back_the_formula_and_not_a_literal() {
    let (mut draft, piece) = front();
    let node = draft.points_cm(piece)[1].0;
    let written = source(&draft, node, Axis::X);
    assert_eq!(written, "cintura / 4 + 1");
    draft.begin_gesture("mover punto");
    draft
        .edit(Command::SetBinding {
            point: node,
            axis: Axis::X,
            to: Binding::parse("cintura / 4 + 1.6").expect("the source parses"),
        })
        .expect("the document takes the binding");
    draft.end_gesture();
    assert_eq!(draft.resolved(node), Some([22.6, 0.0]));
    draft.undo().expect("the entry comes back out");
    assert_eq!(source(&draft, node, Axis::X), written);
    draft.redo().expect("the entry goes back in");
    assert_eq!(source(&draft, node, Axis::X), "cintura / 4 + 1.6");
}

#[test]
fn a_click_that_edits_nothing_leaves_no_entry() {
    let (mut draft, _) = front();
    draft.begin_gesture("mover punto");
    draft.end_gesture();
    assert_eq!(draft.undo_depth(), 0);
    assert_eq!(draft.undo(), Ok(Recompile::Shape(Vec::new())));
}
