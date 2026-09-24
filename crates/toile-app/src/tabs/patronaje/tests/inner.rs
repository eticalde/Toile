use eframe::egui::epaint::Shape;
use eframe::egui::{Event, Key, Modifiers, Pos2, pos2};
use toile_engine::draft::{Doc, LineKey, LineKind, PieceKey, PointKey, block};

use super::super::chalk;
use super::super::gesture::Gesture;
use super::super::inspector::inner::{erase_id, kind_id, row_id};
use super::super::state::{Scope, Selection, Tool};
use super::sewing::says;
use super::studio::{Studio, painted};

/// Where the mat is bare: well past the right of the block, and clear of every
/// contour, so a hover there lights nothing and measures nothing.
const BARE: Pos2 = pos2(1000.0, 700.0);

/// The shipped front alone on the mat, where its lines are drawn and traced.
fn detail(doc: Doc) -> (Studio, PieceKey) {
    let mut studio = Studio::new(doc);
    let piece = studio.doc().piece_keys()[0];
    studio.state.scope = Scope::Piece;
    studio.state.active = Some(piece);
    studio.frame(Vec::new());
    (studio, piece)
}

/// Two named nodes of the piece, where the mat draws them right now.
fn named(studio: &Studio, piece: PieceKey, labels: [&str; 2]) -> [PointKey; 2] {
    labels.map(|label| {
        studio
            .doc()
            .shows_label(piece, label)
            .unwrap_or_else(|| panic!("the block names {label}"))
    })
}

/// The one line the product holds, with its key.
fn only(studio: &Studio) -> (LineKey, LineKind) {
    let doc = studio.doc();
    assert_eq!(doc.lines.len(), 1, "one line, no more");
    let (key, held) = doc.lines.iter().next().expect("the arena holds one");
    (key, held.kind)
}

/// How many segments the last frame drew in the ink a fold is marked with,
/// chosen or not: a line nobody chose is drawn in the same ink, fainter.
fn folds(studio: &Studio) -> usize {
    let lit = studio.theme.measure;
    let faint = lit.gamma_multiply(chalk::FAINT);
    painted(studio)
        .into_iter()
        .filter(|shape| {
            matches!(shape, Shape::LineSegment { stroke, .. }
                if stroke.color == lit || stroke.color == faint)
        })
        .count()
}

/// The tool in hand and a line traced between two nodes of the front, finished
/// with Enter.
fn traced() -> (Studio, PieceKey) {
    let (mut studio, piece) = detail(block::trouser_front());
    studio.key(Key::L, Modifiers::NONE);
    assert_eq!(studio.state.tool, Tool::Trace, "L takes the line tool");
    let [head, tail] = named(&studio, piece, ["cintura_cf", "cadera_lat"]);
    studio.click(studio.on_glass(head));
    assert!(matches!(studio.state.gesture, Gesture::Tracing(_)));
    assert_eq!(studio.doc().lines.len(), 0, "one place is not a line");
    studio.click(studio.on_glass(tail));
    studio.key(Key::Enter, Modifiers::NONE);
    studio.frame(Vec::new());
    (studio, piece)
}

/// Two presses and Enter draw one line, as one entry, and the panel opens on
/// it.
#[test]
fn the_line_tool_traces_a_line_across_the_piece_in_one_entry() {
    let (studio, piece) = traced();
    let (key, kind) = only(&studio);
    let [head, tail] = named(&studio, piece, ["cintura_cf", "cadera_lat"]);
    let doc = studio.doc();
    let held = doc.lines.get(key).expect("the line is live");
    assert_eq!(held.piece, piece);
    let places: Vec<PointKey> = held
        .vertices()
        .map(|it| it.anchor().expect("on the contour").from)
        .collect();
    assert_eq!(places, [head, tail]);
    assert_eq!(kind, LineKind::Reference, "nothing said about it yet");
    assert_eq!(studio.session.undo_label(), Some("trazar línea"));
    assert_eq!(studio.state.gesture, Gesture::Idle);
    assert_eq!(studio.state.selection, Selection::Line(key), "and chosen");
    assert!(says(&studio, "LÍNEA 1"), "the inspector opens on it");
    assert!(says(&studio, "referencia"), "under the kind it was given");
}

/// The mat draws the line in the stroke its kind asks for, and stops drawing it
/// when the line goes.
#[test]
fn the_mat_draws_the_line_in_the_stroke_its_kind_asks_for() {
    let (mut studio, _) = traced();
    let (key, _) = only(&studio);
    studio.frame(vec![Event::PointerMoved(BARE)]);
    let bare = folds(&studio);

    // A fold is the one kind drawn in the ink of a guide, and dashed, so the
    // segments it leaves are the mat's own answer that the line is on the
    // paper.
    studio.click(studio.centre(kind_id(key)));
    studio.frame(vec![Event::PointerMoved(BARE)]);
    assert_eq!(only(&studio).1, LineKind::Fold);
    let drawn = folds(&studio);
    assert!(drawn > bare + 1, "a dashed fold: {drawn} against {bare}");

    studio.session.undo().expect("the step back");
    studio.session.undo().expect("and the line with it");
    studio.frame(vec![Event::PointerMoved(BARE)]);
    assert_eq!(studio.doc().lines.len(), 0);
    assert_eq!(folds(&studio), bare, "and the paper is bare again");
}

/// Delete rubs the chosen line out as the one entry the panel's press leaves,
/// and undo draws the same line again and chooses it.
#[test]
fn delete_rubs_out_the_chosen_line_and_undo_draws_it_again() {
    let (mut studio, _) = traced();
    let (key, kind) = only(&studio);
    studio.key(Key::Delete, Modifiers::NONE);
    studio.frame(Vec::new());
    assert_eq!(studio.doc().lines.len(), 0, "the piece is bare");
    assert_eq!(studio.session.undo_label(), Some("borrar línea"));
    assert_eq!(studio.state.selection, Selection::None);

    studio.key(Key::Z, Modifiers::COMMAND);
    studio.frame(Vec::new());
    assert_eq!(only(&studio), (key, kind), "the same line, key and all");
    assert_eq!(studio.state.selection, Selection::Line(key), "and chosen");
}

/// A press on the line chooses it as a press on its row does, a second press
/// lets go of it, and neither writes anything.
#[test]
fn a_press_on_a_line_chooses_it_and_writes_nothing() {
    let (mut studio, _) = traced();
    let (key, _) = only(&studio);
    studio.key(Key::V, Modifiers::NONE);
    studio.frame(Vec::new());
    studio.click(studio.centre(row_id(key)));
    studio.frame(Vec::new());
    assert_eq!(studio.state.selection, Selection::None, "let go of first");
    let revision = studio.session.revision();

    let run = &super::super::inner::on(
        studio.session.draft().expect("a product is open"),
        studio.state.active.expect("a piece is in front"),
    )[0]
    .run
    .clone();
    let middle = [0, 1].map(|axis| f64::midpoint(run[0][axis], run[1][axis]));
    studio.click(studio.state.view.to_screen(middle));
    studio.frame(Vec::new());
    assert_eq!(studio.state.selection, Selection::Line(key));
    assert_eq!(
        studio.session.revision(),
        revision,
        "choosing writes nothing"
    );
}

/// Nothing is written by looking: opening a product that carries a line,
/// reading its row, hovering the mat and abandoning a half-traced line all
/// leave the file the file that was opened, and the revision where it stood.
#[test]
fn looking_at_a_line_and_abandoning_a_traced_one_write_nothing() {
    let (doc, key) =
        super::super::inner::tests::drawn(LineKind::Slit, ["cintura_cf", "cadera_lat"]);
    let shipped = doc.to_canonical_json();
    let (mut studio, piece) = detail(doc);
    assert!(says(&studio, "ranura"), "the line is listed under its kind");
    studio.click(studio.centre(row_id(key)));
    studio.frame(Vec::new());
    assert_eq!(studio.state.selection, Selection::Line(key));
    assert!(studio.ctx.read_response(erase_id(key)).is_some());

    studio.key(Key::L, Modifiers::NONE);
    let [head, tail] = named(&studio, piece, ["cintura_lat", "rodilla_lat"]);
    studio.click(studio.on_glass(head));
    studio.frame(vec![Event::PointerMoved(studio.on_glass(tail))]);
    assert!(matches!(studio.state.gesture, Gesture::Tracing(_)));
    studio.key(Key::Escape, Modifiers::NONE);
    assert_eq!(studio.state.gesture, Gesture::Idle, "the line is let go of");
    assert_eq!(studio.state.tool, Tool::Trace, "the tool is still in hand");

    assert_eq!(studio.session.revision(), 0, "no edit was played");
    assert!(
        !studio.session.can_undo(),
        "and nothing reached the history"
    );
    assert_eq!(studio.doc().to_canonical_json(), shipped);
    assert_eq!(
        studio.state.scope,
        Scope::Piece,
        "and nobody left the piece"
    );
}

/// The whole product draws a piece's lines too: a mark is on the cloth wherever
/// the cloth is being looked at, and the overview is where a product opens.
#[test]
fn the_whole_product_draws_the_lines_of_its_pieces() {
    let (mut studio, _) = traced();
    studio.click(studio.centre(kind_id(only(&studio).0)));
    studio.state.overview();
    studio.frame(vec![Event::PointerMoved(BARE)]);
    assert_eq!(studio.state.scope, Scope::Product);
    let drawn = folds(&studio);

    studio.session.undo().expect("the step back");
    studio.session.undo().expect("and the line with it");
    studio.frame(vec![Event::PointerMoved(BARE)]);
    let bare = folds(&studio);
    assert!(drawn > bare + 1, "a dashed fold: {drawn} against {bare}");
}
