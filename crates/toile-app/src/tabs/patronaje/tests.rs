use eframe::egui::{Pos2, Rect, pos2, vec2};
use toile_engine::draft::{Axis, Binding, Command, Doc, MeasureSet, Piece, Point, Winding, block};

use super::*;
use crate::theme::Theme;

/// One press or release of the primary button, where the pointer is.
fn button(at: Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    }
}

/// Clicks the product tree at `at` and hands back what it asked for.
///
/// Three passes, because egui hit-tests against the rects the last one
/// allocated: the row has to have been drawn before it can be pressed.
fn click_the_tree(
    draft: Option<&Draft>,
    renaming: &mut Option<(PieceKey, String)>,
    at: Pos2,
) -> Option<tree::Plea> {
    let ctx = egui::Context::default();
    let theme = Theme::sastreria();
    theme.apply(&ctx);
    let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1320.0, 780.0));
    let mut seen = None;
    for events in [
        vec![egui::Event::PointerMoved(at)],
        vec![button(at, true)],
        vec![button(at, false)],
    ] {
        let input = egui::RawInput {
            screen_rect: Some(screen),
            events,
            ..Default::default()
        };
        let pass = ctx.run_ui(input, |ui| {
            let plea = left_panel(ui, &theme, |ui| {
                tree::product(ui, &theme, draft, None, renaming, false, Scope::Product)
            });
            seen = plea.or(seen.take());
        });
        pass.drop_without_applying_deltas();
    }
    seen
}

/// The row that starts a piece answers whether or not a product is under it.
///
/// Two rows painted the same — one live over a document, one inert over
/// nothing — are the same row to anyone looking at the panel, and at launch
/// there is no document, so the one a person presses is the inert one.
#[test]
fn the_row_that_starts_a_piece_answers_a_click_on_an_empty_table() {
    // With no pieces above it the row sits right under the "Producto" caption:
    // 34 pt of section, 3 pt of spacing, half of a 26 pt row.
    let mut renaming = None;
    assert_eq!(
        click_the_tree(None, &mut renaming, pos2(100.0, 50.0)),
        Some(tree::Plea::Draw),
        "the row asks for a piece with nothing on the table too"
    );

    // And two rows lower down, with a product under it — the whole product and
    // its one piece: the state that always worked, kept working.
    let draft = Draft::from_doc(block::trouser_front()).expect("the block resolves");
    assert_eq!(
        click_the_tree(Some(&draft), &mut renaming, pos2(100.0, 108.0)),
        Some(tree::Plea::Draw),
        "the rows above it move it down, they do not silence it"
    );
}

/// A name being typed outlives the click that ends it, wherever that click
/// lands.
///
/// The click that leaves the field is the click that presses the row under it,
/// so both arrive on the same frame and the tree may only answer one of them.
#[test]
fn the_row_that_starts_a_piece_does_not_swallow_a_rename() {
    let draft = Draft::from_doc(block::trouser_front()).expect("the block resolves");
    let piece = draft.doc().piece_keys()[0];
    let mut renaming = Some((piece, "Delantero izquierdo".to_owned()));
    assert_eq!(
        click_the_tree(Some(&draft), &mut renaming, pos2(100.0, 108.0)),
        Some(tree::Plea::Rename(piece, "Delantero izquierdo".to_owned())),
        "the name typed reaches the document; the press can be repeated"
    );
}

/// The plea has two answers, and the one over an empty table is what lets the
/// row be live there at all: it asks for the product a piece would belong to,
/// instead of opening a drawing gesture over nothing.
#[test]
fn the_plus_row_asks_for_a_product_before_it_opens_a_drawing() {
    let mut empty = State::default();
    begin_piece(&mut empty, false);
    assert_eq!(
        empty.asked,
        Some(Action::New),
        "with no document the press asks the application for a product"
    );
    assert_eq!(
        empty.gesture,
        Gesture::Idle,
        "and opens nothing on a mat with nothing to draw on"
    );

    let mut placed = State::default();
    begin_piece(&mut placed, true);
    assert_eq!(placed.asked, None, "with a product there is nothing to ask");
    assert_eq!(
        placed.gesture,
        Gesture::Drawing {
            pending: Vec::new(),
            rubber: [0.0, 0.0],
        },
        "the mat is left waiting for the first vertex"
    );
}

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
    assert_eq!(report::side_cell(&draft, piece), "lateral 104.6 cm");
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
    assert_eq!(report::side_cell(&draft, piece), "perímetro 60.0 cm");
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

mod bench;
mod roads;
mod whole;
