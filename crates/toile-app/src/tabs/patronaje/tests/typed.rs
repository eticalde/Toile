use eframe::egui::{Key, Modifiers, Pos2};
use toile_engine::draft::{Axis, PieceKey, PointKey, block};

use super::super::inspector::write::id_of;
use super::super::state::{Field, Scope, Selection};
use super::bench::front_and_back;
use super::studio::{FRONT_ROW, Studio, TRAIL, WHOLE_ROW};

/// The trousers on the whole tab, open on the front with the hip chosen, and
/// `text` typed over the hip's X without being confirmed.
///
/// Hands back the front, the hip, and what the document binds its X to.
fn half_written(text: &str) -> (Studio, PieceKey, PointKey, String) {
    let mut studio = Studio::new(block::trousers());
    let (front, _) = front_and_back(studio.doc());
    let hip = studio
        .doc()
        .shows_label(front, "cadera_lat")
        .expect("the block names the hip");
    studio.state.open(front);
    studio.state.selection = Selection::point(hip);
    studio.frame(Vec::new());
    let written = x_of(&studio, hip);
    studio.click(studio.centre(id_of(&Field::Coordinate(hip, Axis::X))));
    studio.key(Key::A, Modifiers::COMMAND);
    studio.text(text);
    let held = studio
        .state
        .editing
        .as_ref()
        .map(|edit| edit.buffer.as_str());
    assert_eq!(held, Some(text), "the box is being written");
    assert_ne!(written, text, "the text is not what the document holds");
    (studio, front, hip, written)
}

/// What the document binds the hip's X to.
fn x_of(studio: &Studio, hip: PointKey) -> String {
    let held = studio.doc().points.get(hip).expect("the hip is live");
    held.binding(Axis::X).source().into_owned()
}

/// A click into the hip's X box and on into its Y box: the X box loses the
/// focus, which is how a row confirms whatever it is holding.
fn in_and_out(studio: &mut Studio, hip: PointKey) {
    studio.click(studio.centre(id_of(&Field::Coordinate(hip, Axis::X))));
    studio.click(studio.centre(id_of(&Field::Coordinate(hip, Axis::Y))));
}

/// Leaving a piece by the tree row or by the trail lets go of a formula half
/// written on it: the box shows the document again when the piece comes back,
/// and a click in and out of it writes nothing.
#[test]
fn a_formula_half_written_on_a_piece_left_by_the_tree_or_the_trail_is_let_go() {
    for (road, leave) in [("tree", WHOLE_ROW), ("trail", TRAIL)] {
        let (mut studio, _, hip, written) = half_written("cadera/4");
        studio.click(leave);
        assert_eq!(studio.state.scope, Scope::Product, "{road}");
        assert_eq!(
            studio.session.revision(),
            0,
            "{road}: leaving confirms nothing"
        );
        assert_eq!(
            studio.state.editing, None,
            "{road}: the text went with the piece"
        );

        studio.click(FRONT_ROW);
        assert_eq!(studio.state.scope, Scope::Piece, "{road}");
        studio.state.selection = Selection::point(hip);
        studio.frame(Vec::new());
        assert_eq!(studio.state.editing, None, "{road}: nothing comes back");
        in_and_out(&mut studio, hip);
        assert_eq!(x_of(&studio, hip), written, "{road}: nothing was written");
    }
}

/// Choosing another node on the mat lets go of a formula half written on the
/// one chosen before.
#[test]
fn a_formula_half_written_on_a_node_is_let_go_when_another_is_chosen() {
    let (mut studio, front, hip, written) = half_written("cadera/4");
    let draft = studio.session.draft().expect("a product is open");
    let other = draft
        .points_cm(front)
        .iter()
        .map(|&(key, _)| key)
        .find(|&key| key != hip)
        .expect("the front has more than one node");
    let (away, back): (Pos2, Pos2) = (studio.on_glass(other), studio.on_glass(hip));
    studio.click(away);
    assert_eq!(studio.state.selection, Selection::point(other));
    assert_eq!(studio.state.editing, None, "the text went with its row");

    studio.click(back);
    assert_eq!(studio.state.selection, Selection::point(hip));
    assert_eq!(studio.state.editing, None, "nothing comes back");
    in_and_out(&mut studio, hip);
    assert_eq!(x_of(&studio, hip), written, "nothing was written");
}
