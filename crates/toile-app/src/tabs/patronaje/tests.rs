use toile_engine::draft::{Axis, Binding, Command, Doc, MeasureSet, Piece, Point, Winding, block};

use super::*;

/// The two edits the table takes in its stride say nothing: a shape edit
/// re-derives, a topology edit goes to the mesher. The one the document
/// refuses has to reach the status bar, because the drawing goes on being
/// edited either way.
#[test]
fn an_edit_the_session_refuses_is_said_in_the_status_bar() {
    let mut session = Session::from_doc(block::trouser_front()).expect("the block drapes");
    let piece = session.piece().expect("the session has a document");
    let node = session
        .draft()
        .expect("the session has a document")
        .points_cm(piece)[0]
        .0;
    let mut state = State::default();

    let moved = Verb::Edit(Box::new(Command::SetBinding {
        point: node,
        axis: Axis::X,
        to: Binding::literal(3.0),
    }));
    assert!(!apply(&mut session, vec![moved], &mut state.refused));
    assert_eq!(
        state.refused, None,
        "a shape edit re-drapes and says nothing"
    );

    let sampled = Verb::Edit(Box::new(Command::SetSamples {
        piece,
        node,
        to: 24,
    }));
    assert!(!apply(&mut session, vec![sampled], &mut state.refused));
    assert!(session.remeshing(), "the rebuild is out with the mesher");

    // A sample count no tract may take: the document refuses it, and the
    // table has to say so.
    let refused = Verb::Edit(Box::new(Command::SetSamples {
        piece,
        node,
        to: 4096,
    }));
    assert!(apply(&mut session, vec![refused], &mut state.refused));
    let cells = status(&session, &state);
    assert!(
        cells
            .iter()
            .any(|(cell, alert)| cell.starts_with("rechazado") && *alert),
        "{cells:?}"
    );
}

#[test]
fn the_side_seam_cell_is_measured_not_quoted() {
    let draft = Draft::from_doc(block::trouser_front()).expect("the block resolves");
    let piece = draft
        .doc()
        .piece_named(block::FRONT)
        .expect("the block draws one piece");
    // Measured along the flattening: the hip is a curve, so the seam is a
    // millimetre longer than the chords through its nodes.
    assert_eq!(side_cell(&draft, piece), "lateral 104.6 cm");
}

#[test]
fn a_piece_that_does_not_name_its_side_reports_its_perimeter() {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
    let corners = [[0.0, 0.0], [10.0, 0.0], [10.0, 20.0], [0.0, 20.0]];
    let points: Vec<_> = corners
        .into_iter()
        .map(|[x, y]| doc.points.insert(Point::at(x, y)))
        .collect();
    let piece = doc
        .pieces
        .insert(Piece::polygon("Cuadro", points, Winding::Cw));
    let draft = Draft::from_doc(doc).expect("a square resolves");
    assert_eq!(side_cell(&draft, piece), "perímetro 60.0 cm");
}

/// One product, two pieces: the mat draws the one chosen in the tree, falls
/// back to the one draping when that choice is gone, and to the first when
/// nothing is chosen — so a piece is always reachable and never stranded.
#[test]
fn the_active_piece_prefers_the_chosen_then_the_draping_then_the_first() {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
    let mut square = |x: f64| {
        let corners = [[x, 0.0], [x + 10.0, 0.0], [x + 10.0, 20.0], [x, 20.0]];
        let points: Vec<_> = corners
            .into_iter()
            .map(|[x, y]| doc.points.insert(Point::at(x, y)))
            .collect();
        doc.pieces
            .insert(Piece::polygon("Pieza", points, Winding::Cw))
    };
    let (first, second) = (square(0.0), square(30.0));
    let draft = Draft::from_doc(doc).expect("two squares resolve");
    let gone = PieceKey::new(9999, 0); // a key the document does not hold

    assert_eq!(
        active_piece(Some(&draft), Some(second), Some(first)),
        Some(second),
        "the chosen piece wins while it exists"
    );
    assert_eq!(
        active_piece(Some(&draft), Some(gone), Some(second)),
        Some(second),
        "a chosen piece the document dropped falls back to the draping one"
    );
    assert_eq!(
        active_piece(Some(&draft), None, None),
        Some(first),
        "with neither chosen nor draping, the first piece is in front"
    );
    assert_eq!(active_piece(None, None, None), None, "no table, no piece");
}
