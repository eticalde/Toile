#![allow(
    clippy::float_cmp,
    reason = "a notch is cut at the fraction the press wrote, exactly"
)]

use eframe::egui::{Key, Modifiers};
use toile_engine::draft::{Notch, NotchCount, NotchKey, PointKey};

use super::super::super::state::Field;
use super::super::notch::{put_on_id, take_off_id};
use super::super::write::id_of;
use super::desk::{Desk, bytes, hem, hip};

/// The one notch the product carries, with its key.
fn only(desk: &Desk) -> (NotchKey, Notch) {
    let doc = desk.session.draft().expect("a product is open").doc();
    assert_eq!(doc.notches.len(), 1, "one notch, no more");
    doc.notches
        .iter()
        .map(|(key, held)| (key, *held))
        .next()
        .expect("the arena holds one")
}

/// A tract with a notch already cut into it.
fn cut() -> (Desk, NotchKey) {
    let (mut desk, node) = hem();
    desk.click(desk.centre(put_on_id(node)));
    desk.frame(Vec::new());
    let (key, _) = only(&desk);
    (desk, key)
}

/// Looking at a tract is not marking it.
///
/// The section draws itself out of the document and offers the press that would
/// cut a notch; until that press the file is the file that was opened, byte for
/// byte.
#[test]
fn a_tract_with_no_notch_offers_one_and_writes_nothing() {
    let (mut desk, node) = hem();
    let opened = bytes(&desk);
    desk.frame(Vec::new());
    assert!(desk.drew(put_on_id(node)), "the section offers a notch");
    assert_eq!(desk.session.revision(), 0, "nothing reached the document");
    assert_eq!(desk.entries(), 0, "and nothing reached the history");
    assert_eq!(bytes(&desk), opened, "the bytes are the bytes it opened");
}

/// The press cuts one notch, at the middle of the chosen tract, in one entry of
/// its own.
///
/// The middle because that is the one place on a tract nobody has to be told
/// about: at a node the mark would sit under the node's own dot.
#[test]
fn the_press_cuts_one_notch_at_the_middle_of_the_tract() {
    let (desk, key) = cut();
    let (_, held) = only(&desk);
    let node = desk.state.selection.edge().expect("a tract is chosen");
    assert_eq!(held.at.from, node, "on the tract that was chosen");
    assert_eq!(held.at.t, 0.5, "half way along it");
    assert_eq!(held.count, NotchCount::Single);
    assert_eq!(held.mate, None, "a lone mark answers to nothing");
    assert_eq!(desk.entries(), 1, "one press, one entry");
    assert_eq!(desk.session.undo_label(), Some("poner un piquete"));
    assert!(desk.drew(take_off_id(key)), "and it offers to take it off");
}

/// Taking it off is one entry, and undo cuts the same notch again under its own
/// key.
#[test]
fn the_notch_comes_off_in_one_entry_and_undo_cuts_it_again() {
    let (mut desk, key) = cut();
    let (_, before) = only(&desk);
    desk.click(desk.centre(take_off_id(key)));
    desk.frame(Vec::new());

    let doc = desk.session.draft().expect("a product is open").doc();
    assert_eq!(doc.notches.len(), 0, "the tract is unmarked again");
    assert_eq!(desk.entries(), 2, "cut, taken off");
    assert_eq!(desk.session.undo_label(), Some("quitar el piquete"));
    let node = desk.state.selection.edge().expect("the tract is chosen");
    desk.frame(Vec::new());
    assert!(desk.drew(put_on_id(node)), "and offers one again");

    desk.session.undo().expect("the removal steps back");
    let (again, after) = only(&desk);
    assert_eq!(again, key, "the same notch, under its own key");
    assert_eq!(after, before);
}

/// Replaces what the «a lo largo» box of `node`'s tract holds with `text`, and
/// confirms it with Enter.
fn write_along(desk: &mut Desk, node: PointKey, text: &str) {
    desk.click(desk.centre(id_of(&Field::Along(node))));
    desk.key(Key::A, Modifiers::COMMAND);
    desk.text(text);
    desk.key(Key::Enter, Modifiers::NONE);
}

/// The fraction typed into the row moves the mark, in one entry of its own.
///
/// This is the one way a mark cut at a node is moved without aiming at it, and
/// the mark the pointer cannot tell from a node is the one every import writes.
#[test]
fn the_fraction_typed_into_the_row_moves_the_mark_in_one_entry() {
    let (mut desk, key) = cut();
    let node = desk.state.selection.edge().expect("a tract is chosen");
    write_along(&mut desk, node, "80");
    let (_, held) = only(&desk);
    assert_eq!(held.at.t, 0.8, "the percentage is the fraction");
    assert_eq!(held.at.from, node, "on the tract it was cut into");
    assert_eq!(desk.entries(), 2, "the press that cut it, and this");
    assert_eq!(desk.session.undo_label(), Some("mover piquete"));

    desk.session.undo().expect("the step back");
    desk.frame(Vec::new());
    assert_eq!(only(&desk).1.at.t, 0.5, "one confirmation, one step back");
    assert_eq!(key, only(&desk).0, "and the same mark throughout");
}

/// A number no tract can answer for never reaches the document, and the box
/// goes on holding what was typed so the missed character can be fixed.
#[test]
fn a_fraction_off_the_end_of_the_tract_is_refused_in_the_row() {
    let (mut desk, _) = cut();
    let node = desk.state.selection.edge().expect("a tract is chosen");
    let revision = desk.session.revision();
    write_along(&mut desk, node, "140");
    assert_eq!(desk.session.revision(), revision, "nothing was written");
    assert_eq!(only(&desk).1.at.t, 0.5, "the mark did not move");
    let edit = desk.state.editing.clone().expect("the box keeps its text");
    assert_eq!(edit.buffer, "140");
    assert_eq!(edit.of, Field::Along(node));
}

/// A click in and out of the row writes nothing at all.
///
/// The box comes up with the mark's own fraction already in it and a box
/// confirms what it holds when the focus leaves, so without a word about it the
/// panel would leave an entry in the history for a mark nobody moved.
#[test]
fn a_click_in_and_out_of_the_row_leaves_the_mark_and_the_history_alone() {
    let (mut desk, _) = cut();
    let node = desk.state.selection.edge().expect("a tract is chosen");
    let entries = desk.entries();
    let opened = bytes(&desk);
    desk.click(desk.centre(id_of(&Field::Along(node))));
    desk.click(desk.centre(take_off_id(only(&desk).0)));
    desk.session.undo().expect("the removal steps back");
    desk.frame(Vec::new());
    assert_eq!(
        desk.entries(),
        entries,
        "the press that cut it, and nothing"
    );
    assert_eq!(bytes(&desk), opened, "the bytes are the bytes it held");
}

/// The section is drawn for a tract and for nothing else.
///
/// A node has no stretch of contour to mark, so a press offering one there
/// could only cut a notch into whichever tract happened to leave it.
#[test]
fn a_node_on_its_own_is_offered_no_notch() {
    let (desk, node) = hip();
    assert!(!desk.drew(put_on_id(node)), "a node carries no notch");
}
