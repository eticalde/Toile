use eframe::egui::text_edit::TextEditState;
use eframe::egui::{CursorIcon, Event, Key, Memory, Modifiers};
use toile_engine::draft::{Axis, PointKey};

use super::super::super::state::{Field, FieldEdit};
use super::super::cite::{measure_id, variable_id};
use super::super::write::id_of;
use super::desk::{Desk, hip};

/// Clicks into the X box of the hip and replaces what it holds with `text`.
fn write_x(desk: &mut Desk, point: PointKey, text: &str) {
    desk.click(desk.centre(id_of(&Field::Coordinate(point, Axis::X))));
    desk.key(Key::A, Modifiers::COMMAND);
    desk.text(text);
}

/// What the panel keeps for the X box of the hip, when it keeps anything.
fn x_buffer(desk: &Desk, point: PointKey) -> Option<FieldEdit> {
    desk.state
        .editing
        .clone()
        .filter(|edit| edit.of == Field::Coordinate(point, Axis::X))
}

/// What the hip's X is bound to and what it resolves to, in the document.
fn x_in_document(desk: &Desk, point: PointKey) -> (String, Option<f64>) {
    let draft = desk.session.draft().expect("a product is open");
    let held = draft.doc().points.get(point).expect("the hip is live");
    let source = held.binding(Axis::X).source().into_owned();
    (source, draft.resolved(point).map(|cm| cm[0]))
}

/// The case the whole feature exists for: a formula half written, a name
/// pressed, and the person still writing.
///
/// Pressing the name takes the focus off the box, and the box confirms on
/// losing it; unhandled, `/4+1` is what would be judged, before the name is in.
#[test]
fn a_name_pressed_while_a_coordinate_is_written_lands_at_the_caret() {
    let (mut desk, point) = hip();
    let x = id_of(&Field::Coordinate(point, Axis::X));
    write_x(&mut desk, point, "/4+1");
    desk.key(Key::Home, Modifiers::NONE);
    desk.click(desk.centre(measure_id("cadera")));

    let edit = x_buffer(&desk, point).expect("the box is still being written");
    assert_eq!(edit.buffer, "cadera/4+1");
    assert!(
        desk.ctx.memory(|m| m.has_focus(x)),
        "the box keeps the focus"
    );
    let caret = TextEditState::load(&desk.ctx, x)
        .and_then(|held| held.cursor.char_range())
        .map(|range| range.primary.index.0);
    assert_eq!(caret, Some(6), "the caret waits right after the name");
    assert_eq!(desk.session.revision(), 0, "nothing reached the document");

    desk.key(Key::Enter, Modifiers::NONE);
    let (source, cm) = x_in_document(&desk, point);
    assert_eq!(source, "cadera/4+1");
    let cadera = desk.session.draft().and_then(|d| d.env().value("cadera"));
    let wanted = cadera.expect("the body carries the hip") / 4.0 + 1.0;
    assert!(cm.is_some_and(|cm| (cm - wanted).abs() < 1e-9), "{cm:?}");
}

/// A formula that already parses is the dangerous one: confirmed on the press,
/// it would be written to the document with the name still missing from it.
#[test]
fn a_name_pressed_over_a_selection_replaces_it_and_confirms_nothing() {
    let (mut desk, point) = hip();
    write_x(&mut desk, point, "cintura/4");
    desk.key(Key::Home, Modifiers::NONE);
    for _ in 0.."cintura".len() {
        desk.key(Key::ArrowRight, Modifiers::SHIFT);
    }
    desk.click(desk.centre(measure_id("cadera")));
    let edit = x_buffer(&desk, point).expect("the box is still being written");
    assert_eq!(edit.buffer, "cadera/4");
    assert_eq!(desk.session.revision(), 0, "cintura/4 was never confirmed");

    desk.text("*2");
    let edit = x_buffer(&desk, point).expect("typing goes on in the same box");
    assert_eq!(
        edit.buffer, "cadera*2/4",
        "right after the name, not at the end"
    );
    desk.key(Key::Enter, Modifiers::NONE);
    assert_eq!(x_in_document(&desk, point).0, "cadera*2/4");
    assert_eq!(desk.session.revision(), 1, "one confirmation, one edit");
}

/// With nothing being written a name is information: it senses no press, does
/// not ask for the hand, and a click on it leaves everything as it was.
#[test]
fn a_name_pressed_while_nothing_is_written_changes_nothing() {
    let (mut desk, point) = hip();
    let cadera = measure_id("cadera");
    let at = desk.centre(cadera);
    desk.frame(vec![Event::PointerMoved(at)]);
    assert_eq!(desk.cursor, CursorIcon::Default);
    let sense = desk.ctx.read_response(cadera).expect("drawn").sense;
    assert!(!sense.senses_click(), "the name senses no press");
    let before = desk.state.clone();
    desk.click(at);
    assert_eq!(desk.state, before);
    assert_eq!(desk.session.revision(), 0);
    assert_eq!(desk.ctx.memory(Memory::focused), None);

    // The same name, once a formula is being written, is a live one.
    write_x(&mut desk, point, "1");
    desk.frame(vec![Event::PointerMoved(at)]);
    assert_eq!(desk.cursor, CursorIcon::PointingHand);
    let sense = desk.ctx.read_response(cadera).expect("drawn").sense;
    assert!(
        sense.senses_click(),
        "while a formula is written it takes a press"
    );
}

/// A variable is offered like a measurement, to every formula but its own:
/// pressed into itself it could only write a cycle.
#[test]
fn a_variable_is_offered_to_every_formula_but_its_own() {
    let (mut desk, _) = hip();
    let doc = desk.session.draft().expect("open").doc();
    let key = |name: &str| {
        doc.variables
            .iter()
            .find(|(_, held)| held.name == name)
            .map(|(key, _)| key)
            .expect("the block names its variables")
    };
    let (ease, width) = (key("holgura_cadera"), key("ancho_bajo"));
    let field = Field::Variable(ease);
    desk.click(desk.centre(id_of(&field)));
    desk.key(Key::A, Modifiers::COMMAND);
    desk.text("/10");
    desk.key(Key::Home, Modifiers::NONE);

    let senses = |desk: &Desk, id| desk.ctx.read_response(id).expect("drawn").sense;
    assert!(!senses(&desk, variable_id(ease)).senses_click());
    assert!(senses(&desk, variable_id(width)).senses_click());
    desk.click(desk.centre(variable_id(width)));
    let edit = desk.state.editing.clone().expect("still being written");
    assert_eq!(
        edit,
        FieldEdit {
            of: field,
            buffer: "ancho_bajo/10".to_owned()
        }
    );
    assert_eq!(desk.session.revision(), 0);

    desk.key(Key::Enter, Modifiers::NONE);
    let value = desk
        .session
        .draft()
        .and_then(|d| d.env().value("holgura_cadera"));
    assert!(value.is_some_and(|cm| (cm - 2.2).abs() < 1e-9), "{value:?}");
}
