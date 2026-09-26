#![allow(
    clippy::float_cmp,
    reason = "a measurement reads back as the very number written into it"
)]

use toile_engine::draft::MeasureSet;
use toile_engine::session::Session;

use super::super::tests::product;
use super::Stand;
use crate::library::tests::Scratch;

/// The tape is laid again when the body is solved again or another row is in
/// hand, and never on a frame that changed neither.
#[test]
fn the_tape_is_laid_only_when_the_body_or_the_row_changes() {
    let (session, mut stand) = product();
    let mesh = stand.rebuild(&session);
    assert!(stand.tape_due(), "a body just solved has no tape on it yet");
    assert_eq!(stand.lay(&mesh), None, "no row in hand, no tape");
    assert!(!stand.tape_due(), "a frame that changed nothing");

    stand.highlight = Some("cintura".to_owned());
    assert!(stand.tape_due(), "another row in hand");
    let waist = stand.lay(&mesh).expect("the waist has a tape");
    assert!(waist.closed, "a girth is a loop");
    assert!(!stand.tape_due(), "the same row, still in hand");

    let mesh = stand.rebuild(&session);
    assert!(stand.tape_due(), "the body moved under the tape");
    assert_eq!(
        stand.lay(&mesh),
        Some(waist),
        "the same body, the same tape"
    );
}

/// With no product open, a slider writes into the loose body, which is the
/// one the fitting reads to decide what the cloth falls on.
#[test]
fn a_slider_with_no_product_open_writes_the_loose_body() {
    let mut session = Session::demo_bodice();
    assert!(session.draft().is_none(), "no product is on the table");
    let mut stand = Stand::default();
    stand.set_measure(&mut session, "cintura", 61.0);
    assert_eq!(stand.loose().get("cintura"), Some(61.0));
}

/// The new-mannequin dialog reaches the same replacement the library's button
/// does, so it leaves the same step back and the same sentence offering it.
#[test]
fn a_body_the_dialog_puts_on_the_stand_leaves_the_same_step_back() {
    let mut session = Session::demo_bodice();
    let mut stand = Stand::default();
    stand.set_measure(&mut session, "cintura", 92.0);
    stand.add(&mut session, MeasureSet::new("Ana", [("cintura", 72.0)]));

    assert_eq!(stand.loose().name, "Ana");
    let said = stand.replaced().expect("the press left its way back");
    assert!(said.contains("Cmd+Z"), "{said}");
    stand.step(&mut session, false);
    assert_eq!(stand.loose().get("cintura"), Some(92.0));
    assert_eq!(stand.replaced(), None, "and only the one step");
}

/// The sentence that offers the step back names a body nobody named the way
/// every other panel names one, rather than leaving a gap between two quotes.
#[test]
fn the_step_back_names_an_unnamed_body_the_way_the_panels_do() {
    let mut session = Session::demo_bodice();
    let mut stand = Stand::default();
    stand.add(&mut session, MeasureSet::default());
    let said = stand.replaced().expect("the press left its way back");
    assert!(
        said.starts_with("«sin nombre» sustituye a «Maniquí»"),
        "{said}"
    );
}

/// Filing the body that took the loose one's place is a write into that body:
/// the library keeps what was written, and the body's link moves to it. The
/// step back throws that link away as it throws a slider away, so the sentence
/// offering it owes the same warning.
#[test]
fn filing_the_body_that_took_its_place_is_said_before_the_step_back() {
    let scratch = Scratch::new("file-then-step-back");
    let mut session = Session::demo_bodice();
    let mut stand = Stand::default();
    stand.rename(&mut session, "Marta");
    stand.add(&mut session, MeasureSet::new("Ana", [("cintura", 72.0)]));
    let said = stand.replaced().expect("the press left its way back");
    assert!(
        !said.contains("descarta"),
        "nothing written into Ana yet: {said}"
    );

    let filed = stand
        .save_to(&mut session, &scratch.library(), "2026-09-26".to_owned())
        .expect("Ana is filed");
    assert_eq!(filed.name, "Ana");
    assert!(
        stand.loose().origin.is_some(),
        "the body is linked to the session it just wrote"
    );
    // The wording of the clause is pinned where the three sliders pin it; what
    // this test owes is that the save reaches the same clause at all.
    let said = stand.replaced().expect("the way back is still open");
    assert!(said.contains("descarta"), "{said}");
    stand.step(&mut session, false);
    assert_eq!(stand.loose().name, "Marta");
}
