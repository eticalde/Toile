use eframe::egui::vec2;
use toile_engine::draft::{Axis, Binding};

use super::tests::{free, glass, table};
use super::*;

#[test]
fn typing_an_exact_number_mid_drag_replaces_the_pointer_value() {
    let table = table();
    let ctx = table.context(free());
    let at = table.on_glass(0);
    let (gesture, _, _) = update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    let (gesture, _, _) = update(
        gesture,
        Input::Move(at + vec2(glass(2.0), 0.0), Mods::default()),
        &ctx,
    );
    let (gesture, _, _) = update(gesture, Input::Key(Key::Tab, Mods::default()), &ctx);
    let (gesture, _, _) = update(gesture, Input::Text("2".to_owned()), &ctx);
    let (gesture, _, _) = update(gesture, Input::Text("5".to_owned()), &ctx);
    let (gesture, _, _) = update(gesture, Input::Key(Key::Backspace, Mods::default()), &ctx);
    let (gesture, _, _) = update(gesture, Input::Text("2.5".to_owned()), &ctx);
    let (gesture, commands, feedback) =
        update(gesture, Input::Key(Key::Enter, Mods::default()), &ctx);
    assert_eq!(gesture, Gesture::Idle);
    assert_eq!(feedback.stack, Some(Stack::Close));
    assert_eq!(
        commands,
        vec![Command::SetBinding {
            point: table.nodes[0].0,
            axis: Axis::X,
            to: Binding::literal(22.5),
        }]
    );
}

#[test]
fn the_precision_box_keeps_text_that_does_not_parse() {
    let table = table();
    let ctx = table.context(free());
    let at = table.on_glass(0);
    let (gesture, _, _) = update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    let (gesture, _, _) = update(gesture, Input::Key(Key::Tab, Mods::default()), &ctx);
    let (gesture, _, _) = update(gesture, Input::Text("2 +".to_owned()), &ctx);
    let (gesture, commands, _) = update(gesture, Input::Key(Key::Enter, Mods::default()), &ctx);
    assert!(commands.is_empty(), "nothing is written until it parses");
    let Gesture::Drag(drag) = &gesture else {
        panic!("the gesture is still in hand: {gesture:?}");
    };
    let typed = drag.typed.as_ref().expect("the box is still open");
    assert_eq!(typed.buffer, "2 +");
}

#[test]
fn the_precision_box_takes_a_formula_too() {
    let table = table();
    let ctx = table.context(free());
    let at = table.on_glass(0);
    let (gesture, _, _) = update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    let (gesture, _, _) = update(gesture, Input::Key(Key::Tab, Mods::default()), &ctx);
    let (gesture, _, _) = update(gesture, Input::Text("cintura / 4".to_owned()), &ctx);
    let (_, commands, _) = update(gesture, Input::Key(Key::Enter, Mods::default()), &ctx);
    let [Command::SetBinding { to, .. }] = commands.as_slice() else {
        panic!("the box writes one binding: {commands:?}");
    };
    assert_eq!(to.source(), "cintura / 4");
}

#[test]
fn escape_closes_the_precision_box_before_it_aborts_the_drag() {
    let table = table();
    let ctx = table.context(free());
    let at = table.on_glass(0);
    let (gesture, _, _) = update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    let (gesture, _, _) = update(
        gesture,
        Input::Move(at + vec2(glass(2.0), 0.0), Mods::default()),
        &ctx,
    );
    let (gesture, _, _) = update(gesture, Input::Key(Key::Tab, Mods::default()), &ctx);
    let (gesture, _, feedback) = update(gesture, Input::Key(Key::Escape, Mods::default()), &ctx);
    assert_eq!(feedback.stack, None, "the drag is still in hand");
    let Gesture::Drag(drag) = &gesture else {
        panic!("the node is still in hand: {gesture:?}");
    };
    assert!(drag.typed.is_none(), "the box is gone");
    let (gesture, _, feedback) = update(gesture, Input::Key(Key::Escape, Mods::default()), &ctx);
    assert_eq!(gesture, Gesture::Idle);
    assert_eq!(feedback.stack, Some(Stack::Cancel));
}
