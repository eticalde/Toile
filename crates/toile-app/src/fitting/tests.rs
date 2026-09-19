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

/// A heavy, muscular male: a body the controls offer, and one that passes
/// through itself where a garment goes.
fn heavy() -> BodyMesh {
    let shape = body::Phenotype {
        gender: 0.0,
        weight: 1.0,
        muscle: 1.0,
        ..body::Phenotype::default()
    };
    body::body_mesh(&shape, &body::NO_LEVERS)
}

/// The field lands first and alone, and only then is the body looked over.
///
/// Somebody is waiting for the field, at launch with the window still hidden,
/// and nobody is waiting to hear where a body crosses itself: so the second
/// is ordered when the first arrives, and never holds it up.
#[test]
fn a_body_is_looked_over_after_its_field_has_landed_and_not_before() {
    let mut fitting = Fitting::empty();
    let mut session = Session::blank(Collider::demo());
    fitting.mesh = Some(heavy());
    let field = Baked {
        key: fitting.key,
        ms: 1.0,
        made: Made::Field(Ok(Collider::demo())),
    };
    assert!(fitting.land(field, &mut session));
    assert!(
        fitting.crossings.is_none(),
        "the field did not wait for them"
    );
    assert!(fitting.wait().is_none(), "what comes next carries no field");
    let found = fitting
        .crossings
        .as_ref()
        .expect("it carries the crossings");
    assert!(found.exposed().count() > 0, "and this body has some");
    assert!(fitting.refused.is_none(), "which refuse nothing");
}

/// Word of a body the fitting has moved past is dropped, like its field.
#[test]
fn crossings_for_a_body_no_longer_on_the_stand_are_dropped() {
    let mut fitting = Fitting::empty();
    let mut session = Session::blank(Collider::demo());
    let stale = Baked {
        key: fitting.key + 1,
        ms: 0.0,
        made: Made::Crossings(toile_engine::body::bake::crossings(&heavy())),
    };
    assert!(!fitting.land(stale, &mut session), "not worth a frame");
    assert!(fitting.crossings.is_none());
}
