#![allow(
    clippy::float_cmp,
    reason = "a measurement reads back as the very number written into it"
)]

mod hover;

use eframe::egui::{self, Rect, pos2, vec2};
use toile_engine::body;
use toile_engine::body::Collider;
use toile_engine::draft::{BodyShape, Command, Doc, MeasureSet};
use toile_engine::session::Session;

use super::stand::{Control, NEW_NAME, Stand};
use super::{identity, measures};
use crate::file::File;
use crate::tabs::{Kept, left_panel, right_panel};
use crate::theme::Theme;

/// The base block Toile ships, open on the table, and the tab over it.
pub(super) fn product() -> (Session, Stand) {
    let session =
        Session::from_doc(File::example(), Collider::demo()).expect("the shipped block drapes");
    (session, Stand::default())
}

/// What the body the pattern resolves against carries for `name`.
pub(super) fn resolved(session: &Session, name: &str) -> Option<f64> {
    session.draft()?.doc().measures()?.get(name)
}

/// The document on the table, as its file would read.
pub(super) fn written(session: &Session) -> String {
    session
        .draft()
        .expect("a product is open")
        .doc()
        .to_canonical_json()
}

/// One drag of a measurement's slider, as the panel plays it: a frame for every
/// value with the pointer down, then the frame the pointer comes off.
pub(super) fn drag(stand: &mut Stand, session: &mut Session, name: &str, values: &[f64]) {
    let control = Control::Measure(name.to_owned());
    for &value in values {
        stand.grip(session, control.clone());
        stand.set_measure(session, name, value);
        stand.keep(&control);
        assert!(!stand.settle(session), "a slider in hand keeps its gesture");
    }
    assert!(
        stand.settle(session),
        "the frame the pointer comes off closes it"
    );
}

/// Both panels drawn over the table for a few frames, with nobody touching
/// them.
fn look(session: &mut Session, stand: &mut Stand) {
    let ctx = egui::Context::default();
    let theme = Theme::sastreria();
    theme.apply(&ctx);
    let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1320.0, 780.0));
    for _ in 0..3 {
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        let pass = ctx.run_ui(input, |ui| {
            left_panel(ui, &theme, |ui| identity::panel(ui, &theme, session, stand));
            right_panel(ui, &theme, |ui| measures::panel(ui, &theme, session, stand));
            stand.settle(session);
        });
        pass.drop_without_applying_deltas();
    }
}

/// A value dragged in the tab is the one the pattern resolves with, and the
/// whole drag is one undo entry that gives the old value back.
#[test]
fn a_measure_dragged_in_the_tab_is_the_one_the_pattern_resolves_with() {
    let (mut session, mut stand) = product();
    assert_eq!(resolved(&session, "cintura"), Some(84.0));

    drag(&mut stand, &mut session, "cintura", &[86.0, 90.0, 92.0]);
    assert_eq!(resolved(&session, "cintura"), Some(92.0));
    let draft = session.draft().expect("a product is open");
    assert_eq!(
        draft.env().value("cintura"),
        Some(92.0),
        "what formulas read"
    );
    assert_eq!(session.undo_label(), Some("editar medida"));

    session.undo().expect("the drag undoes");
    assert_eq!(resolved(&session, "cintura"), Some(84.0));
    assert!(!session.can_undo(), "three frames of drag were one entry");
}

/// A shape given in the tab is kept by the product: the file carries it, and
/// the same file reopened gives the tab the same body.
#[test]
fn a_shape_given_in_the_tab_survives_saving_and_reopening() {
    let (mut session, mut stand) = product();
    let shape = BodyShape {
        sex: 1.0,
        age_years: 41.0,
        build: 0.3,
        ..BodyShape::default()
    };
    stand.grip(&mut session, Control::Shape);
    stand.set_shape(&mut session, shape);
    assert!(stand.settle(&mut session));

    let file = written(&session);
    assert!(
        file.starts_with("{\n  \"toile\": 2,"),
        "a shaped body is version 2"
    );
    let doc = Doc::from_json(&file).expect("what the product writes it reads back");
    let reopened = Session::from_doc(doc, Collider::demo()).expect("the shaped block drapes");
    assert_eq!(Stand::default().shape(&reopened), shape);

    session.undo().expect("the shape undoes");
    assert_eq!(stand.shape(&session), BodyShape::default());
}

/// Opening a product whose body stores no shape and looking at it in the tab
/// writes nothing: the revision holds, and so do the bytes.
#[test]
fn looking_at_a_product_that_stores_no_shape_leaves_it_untouched() {
    let (mut session, mut stand) = product();
    let before = written(&session);
    look(&mut session, &mut stand);
    assert_eq!(stand.shape(&session), BodyShape::default());

    // A press that lands on the value a control already shows moves nothing.
    stand.grip(&mut session, Control::Shape);
    stand.set_shape(&mut session, BodyShape::default());
    stand.settle(&mut session);

    assert_eq!(session.revision(), 0);
    assert!(!session.can_undo());
    assert_eq!(written(&session), before);
}

/// A new body joins the product as the one its pattern resolves with, a second
/// is offered a name of its own, and one undo takes the body and the choice
/// back together.
#[test]
fn a_new_mannequin_joins_the_product_as_the_body_it_resolves_with() {
    let (mut session, mut stand) = product();
    let first = session
        .draft()
        .expect("a product is open")
        .doc()
        .resolve_with;
    let name = Stand::free_name(&session, NEW_NAME);

    // The whole reference tape, as the dialog makes it: the block's variables
    // read more of the body than the waist.
    let mut tape = body::default_measures();
    tape.name.clone_from(&name);
    tape.values.insert("cintura".to_owned(), 72.0);
    stand.add(&mut session, tape);
    assert_eq!(resolved(&session, "cintura"), Some(72.0));
    assert_eq!(stand.body(&session).name, NEW_NAME);
    assert_eq!(session.undo_label(), Some("nuevo maniquí"));
    assert_eq!(Stand::free_name(&session, NEW_NAME), "Maniquí 2");

    session.undo().expect("the addition undoes");
    let doc = session.draft().expect("a product is open").doc();
    assert_eq!(doc.resolve_with, first);
    assert_eq!(doc.mannequin_named(&name), None);
    assert!(
        !session.can_undo(),
        "the body and the choice were one entry"
    );
}

/// A body the pattern cannot resolve with is refused whole: it does not stay
/// in the product unchosen, and leaves nothing to undo.
#[test]
fn a_new_mannequin_the_pattern_cannot_resolve_with_leaves_nothing_behind() {
    let (mut session, mut stand) = product();
    let first = session
        .draft()
        .expect("a product is open")
        .doc()
        .resolve_with;
    stand.add(&mut session, MeasureSet::new("Ana", [("cintura", 72.0)]));
    assert!(stand.refused(&session).is_some(), "the block reads a hip");
    // The slot the addition opened stays issued, as an undone addition's
    // always does; the bodies, and the choice between them, are as they were.
    let doc = session.draft().expect("a product is open").doc();
    assert_eq!((doc.mannequins.len(), doc.resolve_with), (2, first));
    assert_eq!(doc.mannequin_named("Ana"), None);
    assert!(!session.can_undo() && !session.can_redo());
}

/// A name another body of the product carries is refused where it is typed,
/// and the body keeps its own.
#[test]
fn a_name_another_body_carries_is_refused_in_the_tab() {
    let (mut session, mut stand) = product();
    stand.rename(&mut session, "Talla 42");
    assert_eq!(stand.body(&session).name, "Etienne");
    assert!(
        stand
            .refused(&session)
            .is_some_and(|why| why.contains("Talla 42"))
    );
    assert_eq!(session.revision(), 0);

    stand.rename(&mut session, "Etienne Q");
    assert_eq!(stand.body(&session).name, "Etienne Q");
    assert_eq!(stand.refused(&session), None);
    assert_eq!(session.undo_label(), Some("renombrar maniquí"));
}

/// The body is solved again when what it is solved from changed, whoever
/// changed it; not for a name, not for an undo back onto the tape it was solved
/// from, and not while a slider is in hand.
#[test]
fn the_body_is_solved_again_only_when_its_tape_or_shape_changed() {
    let (mut session, mut stand) = product();
    assert!(stand.due(&session), "nothing was built yet");
    let _ = stand.rebuild(&session);
    assert!(!stand.due(&session));

    stand.rename(&mut session, "Etienne Q");
    assert!(!stand.due(&session), "a name is not solved");

    // The edit Patronaje's inspector makes, from outside the tab.
    let mannequin = session
        .draft()
        .expect("a product is open")
        .doc()
        .resolve_with;
    let edit = Command::SetMeasure {
        mannequin,
        name: "cintura".to_owned(),
        to: 90.0,
    };
    session.edit(edit).expect("the body carries a waist");
    assert!(stand.due(&session));
    session.undo().expect("the edit undoes");
    assert!(!stand.due(&session), "back on the tape it was solved from");

    let other = session
        .draft()
        .and_then(|draft| draft.doc().mannequin_named("Talla 42"))
        .expect("the block carries a second body");
    session
        .edit(Command::ResolveWith { mannequin: other })
        .expect("the body is live");
    stand.grip(&mut session, Control::Measure("cintura".to_owned()));
    assert!(!stand.due(&session), "never while a control is in hand");
    stand.settle(&mut session);
    assert!(stand.due(&session), "another body chosen");

    session = Session::blank(Collider::demo());
    let _ = stand.rebuild(&session);
    session =
        Session::from_doc(File::example(), Collider::demo()).expect("the shipped block drapes");
    assert!(stand.due(&session), "another product opened at revision 0");
}

/// With no product on the table the tab still shapes a body, in memory, says
/// that nothing keeps it, and writes nowhere.
#[test]
fn with_no_product_the_tab_shapes_a_body_nothing_keeps() {
    let mut session = Session::demo_bodice();
    let mut stand = Stand::default();
    assert_eq!(Stand::kept(&session), Kept::Nowhere);

    drag(&mut stand, &mut session, "cintura", &[92.0]);
    assert_eq!(stand.body(&session).get("cintura"), Some(92.0));
    stand.add(&mut session, MeasureSet::new("Ana", [("cintura", 72.0)]));
    assert_eq!(stand.body(&session).name, "Ana");
    assert!(session.draft().is_none());
    assert_eq!(session.revision(), 0);
}

/// Cmd+Z in this tab steps the product's history the way it does on the mat:
/// a slider dragged here comes back with one step, and forward with one.
#[test]
fn the_tab_steps_the_products_history_both_ways() {
    let (mut session, mut stand) = product();
    let before = resolved(&session, "cintura");
    drag(&mut stand, &mut session, "cintura", &[88.0, 92.0]);
    stand.step(&mut session, false);
    assert_eq!(resolved(&session, "cintura"), before);
    stand.step(&mut session, true);
    assert_eq!(resolved(&session, "cintura"), Some(92.0));
}
