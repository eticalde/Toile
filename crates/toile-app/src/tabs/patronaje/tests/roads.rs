use eframe::egui::{Key, pos2};
use toile_engine::draft::{Draft, block};

use super::super::state::{Scope, Selection};
use super::super::{State, apply, follow, plead, tree};
use super::bench::{Bench, front_and_back, two_squares};
use super::click_the_tree;

/// A piece row opens that piece on its own, framed; the row of the whole
/// product goes back to every piece.
#[test]
fn a_piece_row_opens_its_piece_and_the_product_row_goes_back() {
    let draft = Draft::from_doc(block::trousers()).expect("the block resolves");
    let (_, back) = front_and_back(draft.doc());
    let mut renaming = None;
    // The whole product sits at 50, the front at 79, the back at 108.
    let row = click_the_tree(Some(&draft), &mut renaming, pos2(100.0, 108.0));
    assert_eq!(row, Some(tree::Plea::Focus(back)));
    let mut state = State {
        frame: false,
        ..State::default()
    };
    let asked = plead(&mut state, tree::Plea::Focus(back), true);
    assert!(asked.is_empty(), "opening a piece edits nothing");
    assert_eq!(state.scope, Scope::Piece);
    assert_eq!(state.active, Some(back));
    assert!(state.frame, "the piece is framed as it opens");

    state.frame = false;
    let row = click_the_tree(Some(&draft), &mut renaming, pos2(100.0, 50.0));
    assert_eq!(row, Some(tree::Plea::Overview));
    assert!(plead(&mut state, tree::Plea::Overview, true).is_empty());
    assert_eq!(state.scope, Scope::Product);
    assert!(state.frame, "every piece is framed on the way back");
    assert_eq!(
        state.active,
        Some(back),
        "the piece last open stays in front"
    );
}

#[test]
fn escape_on_a_piece_lets_go_of_what_is_chosen_then_of_the_piece() {
    let mut bench = Bench::new(block::trousers());
    let (front, _) = front_and_back(bench.doc());
    bench.state.open(front);
    bench.frame(Vec::new());
    let node = bench.session.draft().expect("open").points_cm(front)[0].0;
    bench.state.selection = Selection::point(node);
    bench.key(Key::Escape);
    assert_eq!(bench.state.scope, Scope::Piece, "a chosen node goes first");
    assert_eq!(bench.state.selection, Selection::None);
    bench.state.frame = false;
    bench.key(Key::Escape);
    assert_eq!(bench.state.scope, Scope::Product);
    assert!(bench.state.frame, "every piece is framed on the way back");
    assert_eq!(bench.session.revision(), 0);
}

#[test]
fn a_piece_drawn_from_the_whole_product_lands_on_its_own_detail() {
    let mut bench = Bench::new(block::trousers());
    bench.frame(Vec::new());
    assert!(plead(&mut bench.state, tree::Plea::Draw, true).is_empty());
    assert_eq!(
        bench.state.scope,
        Scope::Piece,
        "a contour is drawn on a piece"
    );
    bench.frame(Vec::new());
    let corners = [
        [60.0, 10.0],
        [80.0, 10.0],
        [80.0, 30.0],
        [60.0, 30.0],
        [60.0, 10.0],
    ];
    for cm in corners {
        let at = bench.state.view.to_screen(cm);
        bench.click(at);
    }
    let drawn = bench
        .doc()
        .piece_named("Pieza 1")
        .expect("a piece was drawn");
    assert_eq!(bench.doc().pieces.len(), 3);
    assert_eq!(bench.state.scope, Scope::Piece);
    assert_eq!(bench.state.active, Some(drawn), "the mat follows the hand");
    assert_eq!(bench.session.undo_label(), Some("dibujar pieza"));
}

#[test]
fn taking_off_the_piece_on_the_mat_goes_back_to_the_whole_product() {
    let mut bench = Bench::new(two_squares());
    let second = bench.doc().piece_keys()[1];
    bench.state.open(second);
    bench.frame(Vec::new());
    let before = bench.doc().piece_keys();
    let verbs = plead(&mut bench.state, tree::Plea::Remove(second), true);
    apply(&mut bench.session, verbs, &mut bench.state.refused);
    assert_eq!(bench.state.refused, None, "the removal is taken");
    follow(&bench.session, &mut bench.state, &before);
    assert!(bench.doc().pieces.get(second).is_none());
    assert_eq!(bench.state.scope, Scope::Product);
}

/// The trail over a piece reads "Producto › Delantero", and its first step is
/// a way back like the tree row and Escape.
#[test]
fn the_first_step_of_the_trail_goes_back_to_the_whole_product() {
    let mut bench = Bench::new(block::trousers());
    let (front, _) = front_and_back(bench.doc());
    bench.state.open(front);
    bench.frame(Vec::new());
    // The step sits where the caption starts, clear of both rulers.
    bench.click(pos2(50.0, 34.0));
    assert_eq!(bench.state.scope, Scope::Product);
    assert_eq!(bench.session.revision(), 0);
}
