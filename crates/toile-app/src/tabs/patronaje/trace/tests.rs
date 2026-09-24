#![allow(
    clippy::float_cmp,
    reason = "a place pressed on a node is that node's own fraction, exactly"
)]

use eframe::egui::{Key, Pos2, vec2};
use toile_engine::draft::{LineEdit, LineKey, PointKey};

use super::*;
use crate::tabs::patronaje::gesture::Mods;
use crate::tabs::patronaje::input::tests::{Table, table};
use crate::tabs::patronaje::state::Tool;
use crate::tabs::patronaje::view::View;

fn down(at: Pos2) -> Input {
    Input::Down(at, Mods::default())
}

fn key(key: Key) -> Input {
    Input::Key(key, Mods::default())
}

/// The tracing a gesture holds, out of whatever the reducer answered with.
fn tracing(gesture: &Gesture) -> &Tracing {
    match gesture {
        Gesture::Tracing(held) => held,
        other => panic!("a line is being traced: {other:?}"),
    }
}

/// The line one command carries, and the places it runs through.
fn drawn(commands: &[Command]) -> (LineEdit, Vec<PointKey>) {
    assert_eq!(commands.len(), 1, "one line, one command: {commands:?}");
    let Command::AddLine { identity, line } = &commands[0] else {
        panic!("the tool draws a line: {:?}", commands[0]);
    };
    assert_eq!(*identity, Identity::New);
    let nodes = line
        .vertices()
        .map(|vertex| vertex.anchor().expect("every place is on the contour").from)
        .collect();
    (*line.clone(), nodes)
}

/// The tool in hand, and a tracing opened on the block's first node.
fn opened(table: &Table) -> Gesture {
    let ctx = table.wielding(Tool::Trace);
    let (gesture, commands, feedback) = begin(table.on_glass(0), &ctx);
    assert!(commands.is_empty(), "one place is not a line");
    assert_eq!(feedback.stack, None, "and nothing has reached the history");
    gesture
}

/// A press on a node anchors the place at that node, so a line drawn between
/// two nodes still follows the cloth when the body changes.
#[test]
fn a_press_on_a_node_puts_the_place_on_that_node() {
    let table = table();
    let held = tracing(&opened(&table)).clone();
    assert_eq!(held.pending.len(), 1);
    assert_eq!(held.pending[0].at.from, table.nodes[0].0);
    assert_eq!(held.pending[0].at.t, 0.0, "on the node, not along a tract");
    assert_eq!(held.rubber, held.pending[0].cm);
}

/// A press that lands on nothing opens nothing: a line is drawn on the cloth of
/// a piece, and the bare mat is not cloth.
#[test]
fn a_press_on_the_bare_mat_opens_no_line() {
    let table = table();
    let ctx = table.wielding(Tool::Trace);
    // Well outside the block, where no node and no tract is within reach.
    let away = View::default().to_screen([-200.0, -200.0]);
    let (gesture, commands, feedback) = begin(away, &ctx);
    assert_eq!(gesture, Gesture::Idle);
    assert!(commands.is_empty());
    assert_eq!(feedback.select, None, "and nothing is chosen either");
}

/// Two places and Enter make the line, as one entry, and the line the inspector
/// opens on is the one just drawn.
#[test]
fn two_places_and_enter_draw_the_line_in_one_entry() {
    let table = table();
    let ctx = table.wielding(Tool::Trace);
    let (gesture, _, _) = update(
        tracing(&opened(&table)).clone(),
        &down(table.on_glass(3)),
        &ctx,
    );
    assert_eq!(tracing(&gesture).pending.len(), 2);
    let (gesture, commands, feedback) = update(tracing(&gesture).clone(), &key(Key::Enter), &ctx);
    assert_eq!(gesture, Gesture::Idle);
    let (edit, nodes) = drawn(&commands);
    assert_eq!(nodes, [table.nodes[0].0, table.nodes[3].0]);
    assert_eq!(edit.piece, table.piece);
    assert_eq!(edit.kind, UNSAID, "nothing has been said about it yet");
    assert_eq!(feedback.stack, Some(Stack::Once(TRACE)));
    let key = LineKey::new(table.draft.doc().lines.issued(), 0);
    assert_eq!(feedback.select, Some(Selection::Line(key)));
}

/// A press back on the place pressed last finishes the line, so the hand needs
/// no key to end one.
#[test]
fn a_press_on_the_place_pressed_last_finishes_the_line() {
    let table = table();
    let ctx = table.wielding(Tool::Trace);
    let second = table.on_glass(3);
    let (gesture, _, _) = update(tracing(&opened(&table)).clone(), &down(second), &ctx);
    let (gesture, commands, feedback) = update(tracing(&gesture).clone(), &down(second), &ctx);
    assert_eq!(gesture, Gesture::Idle);
    let (_, nodes) = drawn(&commands);
    assert_eq!(nodes, [table.nodes[0].0, table.nodes[3].0]);
    assert_eq!(feedback.stack, Some(Stack::Once(TRACE)));
}

/// Escape abandons however many places were pressed and writes nothing at all:
/// nothing of a tracing has reached the document, so there is no entry to
/// unwind.
#[test]
fn escape_abandons_the_line_and_writes_nothing() {
    let table = table();
    let ctx = table.wielding(Tool::Trace);
    let (gesture, _, _) = update(
        tracing(&opened(&table)).clone(),
        &down(table.on_glass(3)),
        &ctx,
    );
    let (gesture, commands, feedback) = update(tracing(&gesture).clone(), &key(Key::Escape), &ctx);
    assert_eq!(gesture, Gesture::Idle);
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(feedback.stack, None, "and nothing was opened to close");
    assert_eq!(feedback.select, None);
}

/// Asking to finish a run of one place keeps the tracing: a line runs from one
/// place to another, so one place is not a line.
#[test]
fn enter_on_a_single_place_leaves_the_line_being_traced() {
    let table = table();
    let ctx = table.wielding(Tool::Trace);
    let (gesture, commands, feedback) =
        update(tracing(&opened(&table)).clone(), &key(Key::Enter), &ctx);
    assert_eq!(tracing(&gesture).pending.len(), 1);
    assert!(commands.is_empty());
    assert_eq!(feedback.stack, None);
}

/// Backspace takes the last place back for free, and the run shortens.
#[test]
fn backspace_takes_the_last_place_back() {
    let table = table();
    let ctx = table.wielding(Tool::Trace);
    let (gesture, _, _) = update(
        tracing(&opened(&table)).clone(),
        &down(table.on_glass(3)),
        &ctx,
    );
    let (gesture, commands, _) = update(tracing(&gesture).clone(), &key(Key::Backspace), &ctx);
    assert_eq!(tracing(&gesture).pending.len(), 1);
    assert!(commands.is_empty(), "nothing was written to take back");
}

/// A press part way along a tract anchors there, at the fraction the pointer
/// landed at, so a pocket mouth need not begin on a corner.
#[test]
fn a_press_along_a_tract_anchors_at_the_fraction_it_landed_at() {
    let table = table();
    let ctx = table.wielding(Tool::Trace);
    let ends = (table.nodes[0].1, table.nodes[1].1);
    let middle = [0, 1].map(|axis| f64::midpoint(ends.0[axis], ends.1[axis]));
    let (gesture, _, _) = begin(View::default().to_screen(middle), &ctx);
    let place = tracing(&gesture).pending[0];
    assert_eq!(place.at.from, table.nodes[0].0, "the tract's own node");
    assert!(place.at.t > 0.2 && place.at.t < 0.8, "{}", place.at.t);
}

/// The pointer moving carries the rubber line, and nothing else.
#[test]
fn the_pointer_moving_only_moves_the_rubber_line() {
    let table = table();
    let ctx = table.wielding(Tool::Trace);
    let held = tracing(&opened(&table)).clone();
    let away = table.on_glass(0) + vec2(300.0, 300.0);
    let (gesture, commands, feedback) =
        update(held.clone(), &Input::Move(away, Mods::default()), &ctx);
    let moved = tracing(&gesture);
    assert_eq!(moved.pending, held.pending, "the places stand still");
    assert_ne!(moved.rubber, held.rubber, "the rubber line follows");
    assert!(commands.is_empty());
    assert_eq!(feedback.stack, None);
}

/// Space held turns the press into the pan the mat already reads it as, and the
/// tracing is kept: sliding the drawing is not abandoning the line.
#[test]
fn a_press_with_space_held_keeps_the_line_being_traced() {
    let table = table();
    let ctx = table.wielding(Tool::Trace);
    let held = tracing(&opened(&table)).clone();
    let mods = Mods {
        space: true,
        ..Mods::default()
    };
    let pressed = Input::Down(table.on_glass(3), mods);
    let (gesture, commands, _) = update(held.clone(), &pressed, &ctx);
    assert_eq!(tracing(&gesture).pending, held.pending);
    assert!(commands.is_empty());
}
