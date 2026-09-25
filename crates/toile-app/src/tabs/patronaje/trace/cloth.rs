#![allow(
    clippy::float_cmp,
    reason = "a place written at the resolution of the snap is that number exactly"
)]

use eframe::egui::Key;
use toile_engine::draft::Binding;

use super::tests::{down, key, on_the_cloth, tracing};
use super::*;
use crate::tabs::patronaje::input::tests::table;
use crate::tabs::patronaje::state::Tool;
use crate::tabs::patronaje::view::View;

/// A press on the bare cloth of the piece opens a place of its own, and that
/// place is written from the node nearest it: its formula, plus the offset.
///
/// That is what a buttonhole, a button and a belt-loop mark are — nothing on
/// the contour says where they go — and inheriting the node's binding is what
/// keeps them meaning something on the next body.
#[test]
fn a_press_on_the_bare_cloth_writes_the_place_from_the_node_nearest_it() {
    let table = table();
    let ctx = table.wielding(Tool::Trace);
    let at = on_the_cloth(&table);
    let (gesture, commands, _) = begin(View::default().to_screen(at), Mods::default(), &ctx);
    assert!(commands.is_empty(), "one place is not a line");
    let held = tracing(&gesture);
    let Spot::Loose(point) = &held.pending[0].at else {
        panic!("the cloth is no contour: {:?}", held.pending[0]);
    };
    assert_eq!(held.pending[0].cm, at, "where the press landed");
    // The centre front is the nearest node: its x is a plain zero and its y is
    // one too, so the place is written as the two plain numbers it landed on.
    assert_eq!(point.x, Binding::literal(at[0]));
    assert_eq!(point.y, Binding::literal(at[1]));
    assert_eq!(point.label, None, "the hand names nothing");
}

/// The same press beside a node whose coordinate is a formula inherits the
/// formula, with the offset absorbed into its adjustment term.
#[test]
fn a_place_beside_a_node_written_as_a_formula_inherits_the_formula() {
    let table = table();
    let ctx = table.wielding(Tool::Trace);
    // Two centimetres in from the side waist, whose x is `cintura / 4 + 1`.
    let waist = table.nodes[1].1;
    let at = [waist[0] - 2.0, waist[1] + 2.0];
    let (gesture, _, _) = begin(View::default().to_screen(at), Mods::default(), &ctx);
    let Spot::Loose(point) = &tracing(&gesture).pending[0].at else {
        panic!("the cloth is no contour");
    };
    assert_eq!(point.x.source(), "cintura / 4 - 1", "the delta is absorbed");
    assert_eq!(point.y, Binding::literal(2.0), "a plain zero stays plain");
}

/// Ctrl puts the ladder out, as it does in every other gesture, so a press
/// lands a place of its own wherever the pointer is — on a node included. A
/// place that lands on a node has moved nowhere from it, so it inherits its
/// formula whole.
#[test]
fn with_the_snap_out_a_press_on_a_node_still_makes_a_place_of_its_own() {
    let table = table();
    let ctx = table.wielding(Tool::Trace);
    let mods = Mods {
        ctrl: true,
        ..Mods::default()
    };
    let (gesture, _, _) = begin(table.on_glass(1), mods, &ctx);
    let Spot::Loose(point) = &tracing(&gesture).pending[0].at else {
        panic!("with nothing catching, the place is its own");
    };
    assert_eq!(
        point.x.source(),
        "cintura / 4 + 1",
        "the node's own formula"
    );
}

/// A line of places on the bare cloth reaches the document as the points it is
/// drawn with, and the whole run is one entry.
#[test]
fn a_line_drawn_on_the_cloth_carries_its_own_points_into_the_document() {
    let table = table();
    let ctx = table.wielding(Tool::Trace);
    let first = on_the_cloth(&table);
    let second = [first[0], first[1] + 2.0];
    let (gesture, _, _) = begin(View::default().to_screen(first), Mods::default(), &ctx);
    let (gesture, _, _) = update(
        tracing(&gesture).clone(),
        &down(View::default().to_screen(second)),
        &ctx,
    );
    assert_eq!(tracing(&gesture).pending.len(), 2);
    let (gesture, commands, feedback) = update(tracing(&gesture).clone(), &key(Key::Enter), &ctx);
    assert_eq!(gesture, Gesture::Idle);
    assert_eq!(commands.len(), 1);
    let Command::AddLine { line, .. } = &commands[0] else {
        panic!("the tool draws a line: {:?}", commands[0]);
    };
    let places: Vec<VertexEdit> = line.vertices().cloned().collect();
    assert!(
        places
            .iter()
            .all(|place| matches!(place, VertexEdit::Free { .. })),
        "{places:?}"
    );
    assert_eq!(feedback.stack, Some(Stack::Once(TRACE)));
}
