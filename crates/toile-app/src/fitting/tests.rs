#![allow(
    clippy::float_cmp,
    reason = "a measurement reads back as the very number written into it"
)]

use super::*;

/// Two seconds of drag at sixty frames a second.
const FRAMES: u32 = 120;

/// With no product on the table, the body the cloth falls on is the very one
/// the mannequin tab shapes.
///
/// The two used to be different bodies: this read the reference tape while
/// every slider in that tab wrote into a loose body of its own, so dragging
/// the waist there moved nothing the drape or the fitting room could see.
#[test]
fn with_no_product_open_the_loose_body_is_the_body() {
    let mut loose = body::default_measures();
    loose.values.insert("cintura".to_owned(), 61.0);
    assert_eq!(tape_of(None, &loose).get("cintura"), Some(61.0));
    assert_ne!(
        body::default_measures().get("cintura"),
        Some(61.0),
        "and 61 is nobody's default, so the answer above came from the loose body"
    );
}

/// A hand still on the slider asks for no body at all.
///
/// Every frame of a drag writes a value, and each one used to solve a mesh on
/// this thread and queue a field of tens of megabytes behind the last — all
/// of them paid for in full, all but one thrown away. The body the hand is on
/// its way to is the only one worth having.
#[test]
fn a_drag_asks_for_no_body_while_the_hand_is_still_on_the_slider() {
    let mut fitting = Fitting::empty();
    let mut session = Session::blank(Collider::demo());
    let mut loose = body::default_measures();
    for frame in 0..FRAMES {
        loose
            .values
            .insert("cintura".to_owned(), 84.0 + f64::from(frame) * 0.1);
        fitting.settle(&mut session, &loose, Hand::Holding);
        // Asked every frame rather than at the end: a fitting that chased the
        // hand bakes a real body here, and this has to fail rather than grind.
        assert_eq!(fitting.solves(), 0, "frame {frame} asked for a body");
    }
}
