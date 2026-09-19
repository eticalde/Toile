use eframe::egui::pos2;
use toile_engine::couture::HOLDS_ITS_RATIO;
use toile_engine::draft::{Doc, Elastic, ElasticKey};

use super::super::elastic::{put_on_id, ratio_id, strength_id, take_off_id};
use super::desk::{Desk, hem, hip};

/// The one elastic the product holds, with its key.
fn only(desk: &Desk) -> (ElasticKey, Elastic) {
    let doc = desk.session.draft().expect("a product is open").doc();
    assert_eq!(doc.elastics.len(), 1, "one elastic, no more");
    doc.elastics
        .iter()
        .map(|(key, held)| (key, *held))
        .next()
        .expect("the arena holds one")
}

/// What the product's file says right now.
fn bytes(desk: &Desk) -> String {
    desk.session
        .draft()
        .expect("a product is open")
        .doc()
        .to_canonical_json()
}

/// A tract with an elastic already on it, at the length it was drawn.
fn held() -> (Desk, ElasticKey) {
    let (mut desk, node) = hem();
    desk.click(desk.centre(put_on_id(node)));
    desk.frame(Vec::new());
    let (key, _) = only(&desk);
    (desk, key)
}

/// Looking at a tract is not editing it.
///
/// The section draws itself out of the document and offers the press that
/// would put an elastic on; until that press the file is the file that was
/// opened, byte for byte, and its version has not moved.
#[test]
fn a_tract_with_no_elastic_offers_one_and_writes_nothing() {
    let shipped = Doc::to_canonical_json(&toile_engine::draft::block::trouser_front());
    let (mut desk, node) = hem();
    desk.frame(Vec::new());
    assert!(desk.drew(put_on_id(node)), "the section offers an elastic");
    assert_eq!(desk.session.revision(), 0, "nothing reached the document");
    assert_eq!(desk.entries(), 0, "and nothing reached the history");
    assert_eq!(bytes(&desk), shipped, "the bytes are the bytes it opened");
}

/// The press puts one on at the very length the tract was drawn at, in one
/// entry of its own.
///
/// Neutral on purpose: the press says the stretch is elastic, and the cloth
/// stays where it stood until somebody pulls it in.
#[test]
fn the_press_puts_an_elastic_on_at_the_drawn_length() {
    let (desk, key) = held();
    let (_, elastic) = only(&desk);
    assert_eq!(elastic.ratio.to_bits(), Elastic::NEUTRAL_RATIO.to_bits());
    assert_eq!(elastic.strength.to_bits(), HOLDS_ITS_RATIO.to_bits());
    assert_eq!(desk.entries(), 1, "one press, one entry");
    assert!(desk.drew(ratio_id(key)), "the rails took its place");
    assert!(desk.drew(strength_id(key)));
}

/// A drag of the ratio rail is one entry, whatever it crossed on the way.
///
/// Every frame of it writes, so that the drawing keeps up with the hand; the
/// entry stays open until the hand lets go, so a single undo puts back the
/// number the rail started from and not the one it passed through last.
#[test]
fn a_drag_of_the_ratio_rail_leaves_one_entry_and_one_undo_takes_it_back() {
    let (mut desk, key) = held();
    let rail = desk.rect(ratio_id(key));
    desk.drag(rail.center(), pos2(rail.left() - 400.0, rail.center().y), 4);
    desk.frame(Vec::new());

    let (_, elastic) = only(&desk);
    assert!(elastic.ratio < 0.25, "pulled hard in: {}", elastic.ratio);
    assert_eq!(desk.entries(), 2, "the whole drag is one entry");
    assert_eq!(
        desk.session.undo_label(),
        Some("ajustar la razón del elástico")
    );

    desk.session.undo().expect("the entry steps back");
    let (_, elastic) = only(&desk);
    assert_eq!(
        elastic.ratio.to_bits(),
        Elastic::NEUTRAL_RATIO.to_bits(),
        "back to where the drag began, not to where it passed"
    );
}

/// The two rails are two entries: the ratio and the strength never share a
/// step of the history.
#[test]
fn the_ratio_and_the_strength_are_two_entries() {
    let (mut desk, key) = held();
    let rail = desk.rect(ratio_id(key));
    desk.drag(rail.center(), pos2(rail.left() - 400.0, rail.center().y), 2);
    let firm = desk.rect(strength_id(key));
    desk.drag(
        firm.center(),
        pos2(firm.right() + 400.0, firm.center().y),
        2,
    );
    desk.frame(Vec::new());

    let (_, elastic) = only(&desk);
    assert!(elastic.strength > 40.0, "firm: {}", elastic.strength);
    assert_eq!(desk.entries(), 3, "put on, tensioned, firmed");
    assert_eq!(
        desk.session.undo_label(),
        Some("ajustar la rigidez del elástico")
    );
    desk.session.undo().expect("the strength steps back");
    let (_, elastic) = only(&desk);
    assert_eq!(elastic.strength.to_bits(), HOLDS_ITS_RATIO.to_bits());
    assert!(elastic.ratio < 0.25, "the ratio is where the drag left it");
}

/// A press on a rail that moves nothing leaves no entry behind.
///
/// The rail answers where the pointer is, so a press landing on the number
/// already stored asks for that very number; writing it would leave a step in
/// the history that undoes to itself.
#[test]
fn a_press_that_lands_on_the_value_already_held_writes_nothing() {
    let (mut desk, key) = held();
    // The first press moves the ratio to whatever sits under it; the second
    // lands on that very number, wherever the layout put the rail.
    let at = desk.rect(ratio_id(key)).center();
    desk.click(at);
    desk.frame(Vec::new());
    let (_, moved) = only(&desk);
    assert_ne!(moved.ratio.to_bits(), Elastic::NEUTRAL_RATIO.to_bits());
    assert_eq!(desk.entries(), 2, "put on, then pressed somewhere else");

    let revision = desk.session.revision();
    desk.click(at);
    desk.frame(Vec::new());
    assert_eq!(desk.entries(), 2, "the second press left no entry");
    assert_eq!(desk.session.revision(), revision, "no edit was played");
    let (_, still) = only(&desk);
    assert_eq!(still.ratio.to_bits(), moved.ratio.to_bits());
}

/// Taking it off is one entry, and undo puts the same elastic back.
#[test]
fn the_elastic_comes_off_in_one_entry_and_undo_puts_it_back() {
    let (mut desk, key) = held();
    let (_, before) = only(&desk);
    desk.click(desk.centre(take_off_id(key)));
    desk.frame(Vec::new());

    let doc = desk.session.draft().expect("a product is open").doc();
    assert_eq!(doc.elastics.len(), 0, "the tract is plain again");
    assert_eq!(desk.entries(), 2, "put on, taken off");
    assert_eq!(desk.session.undo_label(), Some("quitar el elástico"));
    let node = desk.state.selection.edge().expect("the tract is chosen");
    desk.frame(Vec::new());
    assert!(desk.drew(put_on_id(node)), "and offers one again");

    desk.session.undo().expect("the removal steps back");
    let (again, after) = only(&desk);
    assert_eq!(again, key, "the same elastic, under its own key");
    assert_eq!(after.ratio.to_bits(), before.ratio.to_bits());
    assert_eq!(after.strength.to_bits(), before.strength.to_bits());
}

/// The section is drawn for a tract and for nothing else.
///
/// A node has no stretch of contour to hold, so a press offering one there
/// could only write an elastic over whichever tract happened to leave it.
#[test]
fn a_node_on_its_own_is_offered_no_elastic() {
    let (desk, node) = hip();
    assert!(!desk.drew(put_on_id(node)), "a node holds no elastic");
}

/// The rows of the section stack instead of landing on one another.
///
/// The bug this guards is the tab's oldest: a row that places a widget inside
/// the room it took and does not take that room back again hands it to the row
/// below, which is then drawn over the top of it.
#[test]
fn the_rails_and_the_press_stack_one_under_the_other() {
    let (desk, key) = held();
    let ratio = desk.rect(ratio_id(key));
    let strength = desk.rect(strength_id(key));
    let off = desk.rect(take_off_id(key));
    assert!(
        ratio.bottom() < strength.top(),
        "the strength rail sits on the ratio one: {ratio:?} {strength:?}"
    );
    assert!(
        strength.bottom() < off.top(),
        "the press sits on the strength rail: {strength:?} {off:?}"
    );
    assert!(
        ratio.width() > 100.0,
        "a rail wide enough to drag: {ratio:?}"
    );
}
