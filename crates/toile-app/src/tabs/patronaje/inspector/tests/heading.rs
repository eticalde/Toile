use toile_engine::draft::{
    FORMAT_VERSION_HEADED, FORMAT_VERSION_HUNG, Hang, HangKey, Heading, PointKey, Sense, block,
};

use super::super::super::state::{Scope, Selection};
use super::super::hang::heading::{
    put_on_id as heading_on_id, sense_id, take_off_id as heading_off_id, turn_id,
};
use super::super::hang::put_on_id;
use super::desk::{Desk, hem};

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

/// A tract hung and turned to the centre front, which is where the press puts
/// it.
fn headed() -> (Desk, HangKey) {
    let (mut desk, key) = held();
    desk.click(desk.centre(heading_on_id(key)));
    desk.frame(Vec::new());
    (desk, key)
}

/// Which way the one hang faces, as the document holds it.
fn facing(desk: &Desk) -> Option<Heading> {
    let doc = desk.session.draft().expect("a product is open").doc();
    doc.hangs.iter().next().and_then(|(_, held)| held.heading)
}

/// A hang nobody turned offers the press that turns it and writes nothing.
///
/// The hang itself is already a version 8 document; until this press it stays
/// one, which is the whole of the conditional rule said in the interface.
#[test]
fn a_hang_nobody_turned_offers_the_press_and_leaves_the_version_where_it_was() {
    let (desk, key) = held();
    assert!(
        desk.drew(heading_on_id(key)),
        "the section offers the rumbo"
    );
    assert!(!desk.drew(turn_id(key)), "and no box until it is pressed");
    assert_eq!(facing(&desk), None);
    let doc = desk.session.draft().expect("a product is open").doc();
    assert_eq!(doc.format_version(), FORMAT_VERSION_HUNG);
}

/// The press turns it to the centre front running toward the wearer's left, in
/// one entry, and that is the version the file then asks for.
#[test]
fn the_press_turns_the_hang_to_the_centre_front() {
    let (mut desk, key) = held();
    desk.click(desk.centre(heading_on_id(key)));
    desk.frame(Vec::new());

    assert_eq!(facing(&desk), Some(Heading::facing(0.0, Sense::Leftward)));
    assert_eq!(desk.entries(), 2, "hung, then turned");
    assert_eq!(desk.session.undo_label(), Some("dar rumbo al colgado"));
    let doc = desk.session.draft().expect("a product is open").doc();
    assert_eq!(doc.format_version(), FORMAT_VERSION_HEADED);
    assert!(desk.drew(turn_id(key)), "the two boxes took their place");
    assert!(desk.drew(sense_id(key)));
    assert!(desk.drew(heading_off_id(key)));
}

/// The turn box steps through the four names of the trade and closes the ring,
/// one press one entry.
///
/// The four and not a slider, because those four are what a person says out
/// loud; the file holds any angle and the boxes reach these.
#[test]
fn the_turn_box_steps_through_the_four_names_of_the_trade() {
    let (mut desk, key) = headed();
    let mut walked = vec![facing(&desk).expect("turned").turn];
    for _ in 1..Heading::QUARTERS.len() {
        desk.click(desk.centre(turn_id(key)));
        desk.frame(Vec::new());
        walked.push(facing(&desk).expect("still turned").turn);
    }
    assert_eq!(walked, [0.0, 90.0, 180.0, -90.0]);
    assert_eq!(desk.entries(), 2 + 3, "hung, turned, three steps");

    desk.click(desk.centre(turn_id(key)));
    desk.frame(Vec::new());
    assert_eq!(facing(&desk).expect("turned").turn, 0.0, "the ring closes");
}

/// The sense box steps to the other side and back, and a step is its own entry.
#[test]
fn the_sense_box_steps_to_the_other_side_of_the_body_and_back() {
    let (mut desk, key) = headed();
    for sense in [Sense::Rightward, Sense::Leftward] {
        desk.click(desk.centre(sense_id(key)));
        desk.frame(Vec::new());
        assert_eq!(facing(&desk).expect("turned").sense, sense);
    }
    assert_eq!(desk.entries(), 2 + 2, "hung, turned, two steps");
    assert_eq!(
        desk.session.undo_label(),
        Some("cambiar el sentido del colgado")
    );
}

/// Taking the rumbo off leaves the hang hanging and the file back at the
/// version it was, and undo puts the very heading back.
#[test]
fn the_rumbo_comes_off_in_one_entry_and_undo_puts_it_back() {
    let (mut desk, key) = headed();
    desk.click(desk.centre(turn_id(key)));
    desk.frame(Vec::new());
    let before = facing(&desk).expect("turned");

    desk.click(desk.centre(heading_off_id(key)));
    desk.frame(Vec::new());
    assert_eq!(facing(&desk), None, "the rumbo is off");
    let (again, station) = only(&desk);
    assert_eq!((again, station.as_str()), (key, Hang::WAIST), "still hung");
    let doc = desk.session.draft().expect("a product is open").doc();
    assert_eq!(doc.format_version(), FORMAT_VERSION_HUNG);
    assert!(desk.drew(heading_on_id(key)), "and offers the press again");

    desk.session.undo().expect("the removal steps back");
    assert_eq!(facing(&desk), Some(before), "the same heading, as it was");
}

/// The rows of the rumbo stack instead of landing on one another.
#[test]
fn the_two_boxes_and_the_press_stack_one_under_the_other() {
    let (desk, key) = headed();
    let (turn, sense, off) = (
        desk.rect(turn_id(key)),
        desk.rect(sense_id(key)),
        desk.rect(heading_off_id(key)),
    );
    assert!(turn.bottom() < sense.top(), "{turn:?} {sense:?}");
    assert!(sense.bottom() < off.top(), "{sense:?} {off:?}");
}

/// The shipped block open on its own centre front: the tract between the
/// crotch point and the waistline, whose two ends stand at one abscissa.
///
/// The edge a pattern cutter means by "centre front", and the edge the press
/// used to write a declaration about that the placement then dropped.
fn centre_front() -> (Desk, PointKey) {
    let doc = block::trouser_front();
    let piece = doc
        .piece_named(block::FRONT)
        .expect("the block draws one piece");
    let point = doc
        .shows_label(piece, "tiro_cf")
        .expect("the block names its crotch point");
    let mut desk = Desk::tall(doc);
    desk.state.scope = Scope::Piece;
    desk.state.active = Some(piece);
    desk.state.selection = Selection::Edge(point);
    desk.frame(Vec::new());
    (desk, point)
}

/// The shipped block's centre front can be turned, and the press that turns it
/// moves the garment.
///
/// What this is about is a press that lied. Both ends of that edge are drawn at
/// abscissa zero, the pin was refused for saying nothing about which way the
/// cloth goes on, and the press was drawn anyway: it wrote a heading, raised
/// the file's version, and the release came out at the same −55.594° it was at
/// before. The edge does say which way the cloth goes on — the cloth is all on
/// one side of it — so the answer is to read that rather than to grey the
/// press out, and the centre front now lands at 0.000°.
#[test]
fn the_centre_front_of_the_shipped_block_can_be_turned() {
    let (mut desk, node) = centre_front();
    desk.click(desk.centre(put_on_id(node)));
    desk.frame(Vec::new());
    let (key, _) = only(&desk);
    assert!(
        desk.session.headless().is_empty(),
        "the centre front takes a heading"
    );
    assert!(desk.drew(heading_on_id(key)), "so the press is drawn");
    let before = turned(&desk);

    desk.click(desk.centre(heading_on_id(key)));
    desk.frame(Vec::new());
    assert_eq!(facing(&desk), Some(Heading::facing(0.0, Sense::Leftward)));
    let after = turned(&desk);
    assert!(
        (after - 0.0).abs() < 1.0e-9,
        "the centre front came out at {after:.3}\u{b0} where {before:.3}\u{b0} \
         was the undeclared release"
    );
    assert!(
        (before - after).abs() > 1.0,
        "and the press moved the garment: {before:.3}\u{b0} to {after:.3}\u{b0}"
    );
}

/// Where the block's own centre front came out, in degrees.
fn turned(desk: &Desk) -> f64 {
    let ring = desk
        .session
        .rings()
        .into_iter()
        .next()
        .expect("the hung panel is placed");
    ring.round
        .facing(0, 0.0, ring.crest)
        .expect("the ring carries the one piece")
}
