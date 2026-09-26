use toile_engine::draft::{Doc, EdgeRange, Hang, HangKey, block};

use super::super::super::state::{Scope, Selection};
use super::super::hang::{put_on_id, station_id, take_off_id};
use super::desk::{Desk, bytes, hem, hip};

/// The one hang the product holds, with its key and the ring it hangs from.
fn only(desk: &Desk) -> (HangKey, String) {
    let doc = desk.session.draft().expect("a product is open").doc();
    assert_eq!(doc.hangs.len(), 1, "one hang, no more");
    doc.hangs
        .iter()
        .map(|(key, held)| (key, held.station.clone()))
        .next()
        .expect("the arena holds one")
}

/// A tract already hung, from the ring the press hangs it from.
fn held() -> (Desk, HangKey) {
    let (mut desk, node) = hem();
    desk.click(desk.centre(put_on_id(node)));
    desk.frame(Vec::new());
    let (key, _) = only(&desk);
    (desk, key)
}

/// Looking at a tract is not hanging it.
///
/// The section draws itself out of the document and offers the press that would
/// hang the tract; until that press the file is the file that was opened, byte
/// for byte, and its version has not moved.
#[test]
fn a_tract_nobody_hung_offers_the_press_and_writes_nothing() {
    let shipped = Doc::to_canonical_json(&toile_engine::draft::block::trouser_front());
    let (mut desk, node) = hem();
    desk.frame(Vec::new());
    assert!(desk.drew(put_on_id(node)), "the section offers the hang");
    assert_eq!(desk.session.revision(), 0, "nothing reached the document");
    assert_eq!(desk.entries(), 0, "and nothing reached the history");
    assert_eq!(bytes(&desk), shipped, "the bytes are the bytes it opened");
}

/// The press hangs the tract from the waist, in one entry of its own.
#[test]
fn the_press_hangs_the_tract_from_the_waist() {
    let (desk, key) = held();
    let (_, station) = only(&desk);
    assert_eq!(station, Hang::WAIST);
    assert_eq!(desk.entries(), 1, "one press, one entry");
    assert!(desk.drew(station_id(key)), "the ring box took its place");
    assert!(desk.drew(take_off_id(key)));
}

/// The box steps the ring, one press one entry, and undo walks back.
#[test]
fn the_box_steps_to_the_next_ring_of_the_body() {
    let (mut desk, key) = held();
    desk.click(desk.centre(station_id(key)));
    desk.frame(Vec::new());

    let (_, station) = only(&desk);
    assert_eq!(station, "cadera", "the ring under the waist");
    assert_eq!(desk.entries(), 2, "hung, then stepped");
    assert_eq!(
        desk.session.undo_label(),
        Some("cambiar el anillo del que cuelga")
    );

    desk.session.undo().expect("the step goes back");
    let (again, back) = only(&desk);
    assert_eq!(again, key, "the same hang, under its own key");
    assert_eq!(back, Hang::WAIST);
}

/// Letting it down is one entry, and undo hangs the same one back up.
#[test]
fn the_hang_comes_off_in_one_entry_and_undo_puts_it_back() {
    let (mut desk, key) = held();
    desk.click(desk.centre(take_off_id(key)));
    desk.frame(Vec::new());

    let doc = desk.session.draft().expect("a product is open").doc();
    assert_eq!(doc.hangs.len(), 0, "the tract hangs from nothing again");
    assert_eq!(desk.entries(), 2, "hung, let down");
    assert_eq!(desk.session.undo_label(), Some("descolgar el tramo"));
    let node = desk.state.selection.edge().expect("the tract is chosen");
    desk.frame(Vec::new());
    assert!(desk.drew(put_on_id(node)), "and offers the press again");

    desk.session.undo().expect("the removal steps back");
    let (again, station) = only(&desk);
    assert_eq!(again, key, "the same hang, under its own key");
    assert_eq!(station, Hang::WAIST);
}

/// The section is drawn for a tract and for nothing else.
///
/// A node has no stretch of contour to hold up, so a press offering one there
/// could only hang whichever tract happened to leave it.
#[test]
fn a_node_on_its_own_is_offered_no_hang() {
    let (desk, node) = hip();
    assert!(!desk.drew(put_on_id(node)), "a node hangs from nothing");
}

/// A hang whose station names no ring of the body is drawn with nothing to
/// press.
///
/// The commands refuse such a station at both doors it could arrive by, so the
/// arena is written straight to reach the state. What is checked is that the
/// panel offers no step it would have to take back, and still lets the tract
/// down.
#[test]
fn a_station_with_no_ring_behind_it_is_drawn_with_nothing_to_step() {
    let mut doc = block::trouser_front();
    let piece = doc.piece_named(block::FRONT).expect("the block draws one");
    let node = |doc: &Doc, label: &str| {
        doc.shows_label(piece, label)
            .expect("the block names its waist")
    };
    let (head, tail) = (node(&doc, "cintura_cf"), node(&doc, "cintura_lat"));
    let key = doc
        .hangs
        .insert(Hang::new(EdgeRange::between(piece, head, tail), "estatura"));

    let mut desk = Desk::tall(doc);
    desk.state.scope = Scope::Piece;
    desk.state.active = Some(piece);
    desk.state.selection = Selection::Edge(head);
    desk.frame(Vec::new());

    assert!(!desk.drew(station_id(key)), "no ring to step to");
    assert!(!desk.drew(put_on_id(head)), "the tract is hung already");
    assert!(desk.drew(take_off_id(key)), "and it can still come down");
    assert_eq!(desk.entries(), 0, "looking at it wrote nothing");
}

/// The rows of the section stack instead of landing on one another.
#[test]
fn the_ring_box_and_the_press_stack_one_under_the_other() {
    let (desk, key) = held();
    let ring = desk.rect(station_id(key));
    let off = desk.rect(take_off_id(key));
    assert!(
        ring.bottom() < off.top(),
        "the press sits on the ring box: {ring:?} {off:?}"
    );
}
