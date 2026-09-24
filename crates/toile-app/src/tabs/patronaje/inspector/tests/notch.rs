#![allow(
    clippy::float_cmp,
    reason = "a notch is cut at the fraction the press wrote, exactly"
)]

use toile_engine::draft::{Notch, NotchCount, NotchKey};

use super::super::notch::{put_on_id, take_off_id};
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

/// The section is drawn for a tract and for nothing else.
///
/// A node has no stretch of contour to mark, so a press offering one there
/// could only cut a notch into whichever tract happened to leave it.
#[test]
fn a_node_on_its_own_is_offered_no_notch() {
    let (desk, node) = hip();
    assert!(!desk.drew(put_on_id(node)), "a node carries no notch");
}
