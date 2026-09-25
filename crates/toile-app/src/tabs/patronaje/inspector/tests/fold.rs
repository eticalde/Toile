#![allow(
    clippy::float_cmp,
    reason = "an axis is written at the fraction the press wrote, exactly"
)]

use toile_engine::draft::{
    Doc, EdgeRange, MeasureSet, Piece, Point, PointKey, SymmetryKey, Winding,
};

use super::super::fold::{put_on_id, take_off_id};
use super::desk::{Desk, bytes, hem, hip, piece_open};

/// The one axis the product carries, with its key.
fn only(desk: &Desk) -> (SymmetryKey, EdgeRange) {
    let doc = desk.session.draft().expect("a product is open").doc();
    assert_eq!(doc.symmetries.len(), 1, "one axis, no more");
    doc.symmetries
        .iter()
        .map(|(key, held)| (key, held.axis))
        .next()
        .expect("the arena holds one")
}

/// A hundred-centimetre square on a tall desk, open on the tract that leaves
/// its third corner: the left side, which a square can honestly be folded on.
///
/// The shipped block cannot: fold it on any of its edges and the halves lie
/// over each other, which is a defect and not a shape.
fn square() -> (Desk, PointKey) {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
    let corners = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
    let points: Vec<PointKey> = corners
        .into_iter()
        .map(|[x, y]| doc.points.insert(Point::at(x, y)))
        .collect();
    doc.pieces
        .insert(Piece::polygon("Pretina", points.clone(), Winding::Cw));
    let (mut desk, _) = piece_open(doc);
    desk.state.selection = super::super::super::state::Selection::Edge(points[3]);
    desk.frame(Vec::new());
    (desk, points[3])
}

/// The square with its left side made the axis.
fn folded() -> (Desk, SymmetryKey) {
    let (mut desk, node) = square();
    desk.click(desk.centre(put_on_id(node)));
    desk.frame(Vec::new());
    let (key, _) = only(&desk);
    (desk, key)
}

/// Looking at a tract is not folding on it.
///
/// The section offers the press and writes nothing until it is made: the file
/// is the file that was opened, byte for byte, and its version with it.
#[test]
fn a_tract_that_is_not_the_axis_offers_the_fold_and_writes_nothing() {
    let (mut desk, node) = square();
    let opened = bytes(&desk);
    desk.frame(Vec::new());
    assert!(desk.drew(put_on_id(node)), "the section offers a fold");
    assert_eq!(desk.session.revision(), 0, "nothing reached the document");
    assert_eq!(desk.entries(), 0, "and nothing reached the history");
    assert_eq!(bytes(&desk), opened, "the bytes are the bytes it opened");
    assert!(opened.starts_with("{\n  \"toile\": 1,"), "{opened}");
}

/// The press folds the piece on the chosen tract, node to node, in one entry.
#[test]
fn the_press_makes_the_chosen_tract_the_axis_in_one_entry() {
    let (desk, _) = folded();
    let (_, axis) = only(&desk);
    let node = desk.state.selection.edge().expect("a tract is chosen");
    assert_eq!(axis.head.from, node, "the tract that was chosen");
    assert_eq!(axis.head.t, 0.0, "node to node, both ends");
    assert_eq!(axis.tail.t, 0.0);
    assert_eq!(desk.entries(), 1, "one press, one entry");
    assert_eq!(desk.session.undo_label(), Some("poner el doblez"));
    // And the file says so: a product with an axis is written at version 7.
    assert!(
        bytes(&desk).starts_with("{\n  \"toile\": 7,"),
        "{}",
        bytes(&desk)
    );
}

/// The cloth the fold makes is what the table now holds: twice the width, with
/// the drawing still the half.
#[test]
fn the_folded_piece_is_cut_at_twice_the_width_it_is_drawn_at() {
    let (desk, _) = folded();
    let draft = desk.session.draft().expect("a product is open");
    let piece = desk.state.active.expect("a piece is in front");
    let span = |at: &[[f64; 2]]| {
        let high = at.iter().map(|p| p[0]).fold(f64::MIN, f64::max);
        high - at.iter().map(|p| p[0]).fold(f64::MAX, f64::min)
    };
    assert!((span(draft.cloth_cm(piece)) - 200.0).abs() < 1.0e-9);
    assert!((span(draft.flat_cm(piece)) - 100.0).abs() < 1.0e-9);
    assert!(draft.cloth(piece).is_some(), "and the panel can say so");
}

/// Unfolding is one entry, and undo folds the same piece again on the same axis
/// under its own key.
#[test]
fn the_fold_comes_off_in_one_entry_and_undo_puts_it_back() {
    let (mut desk, key) = folded();
    let (_, before) = only(&desk);
    desk.click(desk.centre(take_off_id(key)));
    desk.frame(Vec::new());

    let doc = desk.session.draft().expect("a product is open").doc();
    assert_eq!(doc.symmetries.len(), 0, "the piece is drawn whole again");
    assert_eq!(desk.entries(), 2, "folded, unfolded");
    assert_eq!(desk.session.undo_label(), Some("quitar el doblez"));
    let node = desk.state.selection.edge().expect("the tract is chosen");
    desk.frame(Vec::new());
    assert!(desk.drew(put_on_id(node)), "and offers the fold again");

    desk.session.undo().expect("the removal steps back");
    let (again, after) = only(&desk);
    assert_eq!(again, key, "the same axis, under its own key");
    assert_eq!(after, before);
}

/// A piece carries one axis, so the other tracts stop offering the press once
/// one of them is the axis: a second would be a quarter piece.
#[test]
fn another_tract_of_a_folded_piece_is_offered_no_second_axis() {
    let (mut desk, _) = folded();
    let nodes = desk
        .session
        .draft()
        .expect("a product is open")
        .points_cm(desk.state.active.expect("a piece is in front"))
        .to_vec();
    desk.state.selection = super::super::super::state::Selection::Edge(nodes[1].0);
    desk.frame(Vec::new());
    assert!(!desk.drew(put_on_id(nodes[1].0)), "one axis is the limit");
}

/// The section is drawn for a tract and for nothing else: a node has no stretch
/// of contour to be the axis.
#[test]
fn a_node_on_its_own_is_offered_no_fold() {
    let (desk, node) = hip();
    assert!(!desk.drew(put_on_id(node)), "a node is not an edge");
    let (desk, node) = hem();
    assert!(desk.drew(put_on_id(node)), "and a tract is");
}
