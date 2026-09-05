use std::collections::BTreeSet;

use eframe::egui::vec2;

use super::tests::{bound_x, chosen, free, glass, table};
use super::*;
use crate::tabs::patronaje::view::View;

/// Shift, held.
fn shift() -> Mods {
    Mods {
        shift: true,
        ..Mods::default()
    }
}

#[test]
fn shift_click_adds_to_the_selection() {
    let table = table();
    let (a, b) = (table.nodes[0].0, table.nodes[1].0);
    let ctx = table.context(free());
    let (_, _, feedback) = update(
        Gesture::Idle,
        Input::Down(table.on_glass(0), Mods::default()),
        &ctx,
    );
    assert_eq!(chosen(&feedback), [a]);

    let ctx = table.holding(free(), Selection::point(a));
    let (gesture, _, feedback) =
        update(Gesture::Idle, Input::Down(table.on_glass(1), shift()), &ctx);
    let mut both = vec![a, b];
    both.sort_by_key(|key| (key.index(), key.generation()));
    assert_eq!(chosen(&feedback), both, "shift adds instead of replacing");
    assert!(
        // Both nodes, and the tangent the waist carries into the hip curve.
        matches!(&gesture, Gesture::Drag(drag) if drag.nodes.len() == 3),
        "both nodes are in hand: {gesture:?}"
    );

    // Shift again on the same node takes it back out, and nothing is in hand.
    let ctx = table.holding(free(), Selection::Points(BTreeSet::from([a, b])));
    let (gesture, _, feedback) =
        update(Gesture::Idle, Input::Down(table.on_glass(1), shift()), &ctx);
    assert_eq!(chosen(&feedback), [a]);
    assert_eq!(gesture, Gesture::Idle);
    assert_eq!(feedback.stack, None, "no gesture, no entry");
}

#[test]
fn dragging_a_group_moves_every_node_by_the_same_delta() {
    let table = table();
    let (a, b) = (table.nodes[0].0, table.nodes[1].0);
    let ctx = table.holding(free(), Selection::Points(BTreeSet::from([a, b])));
    let at = table.on_glass(0);
    let (gesture, _, _) = update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    let (_, commands, _) = update(
        gesture,
        Input::Move(at + vec2(glass(2.0), 0.0), Mods::default()),
        &ctx,
    );
    // Two nodes, and the handle the second of them hangs the hip curve on.
    assert_eq!(commands.len(), 3, "one move per point in hand");
    for (command, start) in commands.iter().zip([table.nodes[0].1, table.nodes[1].1]) {
        let x = bound_x(command, &table.draft);
        assert!(
            (x - (start[0] + 2.0)).abs() < 1.0e-9,
            "every node takes the same delta: {x} vs {}",
            start[0] + 2.0
        );
    }
}

#[test]
fn a_marquee_chooses_every_node_it_covers() {
    let table = table();
    let ctx = table.context(free());
    let view = View::default();
    let away = view.to_screen([-40.0, -40.0]);
    let (gesture, _, feedback) = update(Gesture::Idle, Input::Down(away, Mods::default()), &ctx);
    assert_eq!(feedback.select, Some(Selection::None));
    assert!(matches!(gesture, Gesture::Marquee { .. }), "{gesture:?}");
    let far = view.to_screen([400.0, 400.0]);
    let (gesture, _, _) = update(gesture, Input::Move(far, Mods::default()), &ctx);
    let (gesture, _, feedback) = update(gesture, Input::Up(far, Mods::default()), &ctx);
    assert_eq!(gesture, Gesture::Idle);
    assert_eq!(
        chosen(&feedback).len(),
        table.nodes.len(),
        "a band over the whole piece takes the whole piece"
    );
}

#[test]
fn space_held_pans_instead_of_sweeping() {
    let table = table();
    let ctx = table.context(free());
    let space = Mods {
        space: true,
        ..Mods::default()
    };
    let away = View::default().to_screen([-40.0, -40.0]);
    let (gesture, _, feedback) = update(Gesture::Idle, Input::Down(away, space), &ctx);
    assert!(matches!(gesture, Gesture::Pan { .. }), "{gesture:?}");
    assert_eq!(feedback.select, None, "a pan chooses nothing");
}

#[test]
fn a_press_on_a_tract_selects_it_and_a_press_on_the_mat_clears_it() {
    let table = table();
    let ctx = table.context(free());
    let view = View::default();
    let (a, b) = (table.nodes[0].1, table.nodes[1].1);
    let middle = view.to_screen([f64::midpoint(a[0], b[0]), f64::midpoint(a[1], b[1])]);
    let (gesture, commands, feedback) =
        update(Gesture::Idle, Input::Down(middle, Mods::default()), &ctx);
    assert!(commands.is_empty());
    assert_eq!(feedback.select, Some(Selection::Edge(table.nodes[0].0)));
    assert!(matches!(gesture, Gesture::Marquee { .. }));
    let away = view.to_screen([-40.0, -40.0]);
    let (_, _, feedback) = update(Gesture::Idle, Input::Down(away, Mods::default()), &ctx);
    assert_eq!(feedback.select, Some(Selection::None));
}

#[test]
fn the_keyboard_reaches_the_undo_stack_and_the_whole_piece() {
    let table = table();
    let ctx = table.context(free());
    let command = Mods {
        command: true,
        ..Mods::default()
    };
    let (_, commands, feedback) = update(Gesture::Idle, Input::Key(Key::Z, command), &ctx);
    assert!(commands.is_empty());
    assert_eq!(feedback.stack, Some(Stack::Undo));
    let both = Mods {
        shift: true,
        ..command
    };
    let (_, _, feedback) = update(Gesture::Idle, Input::Key(Key::Z, both), &ctx);
    assert_eq!(feedback.stack, Some(Stack::Redo));
    let (_, _, feedback) = update(Gesture::Idle, Input::Key(Key::A, command), &ctx);
    assert_eq!(chosen(&feedback).len(), table.nodes.len());
    let (_, _, feedback) = update(
        Gesture::Idle,
        Input::Key(Key::Escape, Mods::default()),
        &ctx,
    );
    assert_eq!(feedback.select, Some(Selection::None));
}
