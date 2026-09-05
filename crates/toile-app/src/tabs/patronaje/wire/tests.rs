use eframe::egui::{Context, Event, Modifiers, PointerButton, Pos2, RawInput, Sense, pos2, vec2};

use super::*;

/// One frame of a bare mat, fed the events a real pointer sends, giving
/// back what the gesture would be handed.
fn frame(ctx: &Context, events: Vec<Event>) -> Vec<Input> {
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))),
        events,
        ..RawInput::default()
    };
    let mut out = Vec::new();
    let mut full = ctx.run_ui(raw, |ui| {
        egui::CentralPanel::no_frame().show(ui, |ui| {
            let size = ui.available_size();
            let (resp, _) = ui.allocate_painter(size, Sense::click_and_drag());
            out = events_of(ui, &resp);
        });
    });
    // Nothing here paints, so the fonts this frame asked for are dropped
    // on purpose rather than uploaded to a texture that does not exist.
    full.textures_delta.clear();
    out
}

fn button(at: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

/// The one thing about this wiring that a unit test of the reducer cannot
/// reach: that egui's own response really does hand over the press on the
/// frame it happens, and the move and the release after it.
#[test]
fn a_press_a_move_and_a_release_reach_the_gesture() {
    let ctx = Context::default();
    let at = pos2(200.0, 200.0);
    frame(&ctx, vec![Event::PointerMoved(at)]);
    let down = frame(&ctx, vec![button(at, true)]);
    assert!(matches!(down.first(), Some(Input::Down(..))), "{down:?}");
    let away = at + vec2(24.0, 6.0);
    let moved = frame(&ctx, vec![Event::PointerMoved(away)]);
    assert!(
        moved.iter().any(|e| matches!(e, Input::Move(..))),
        "{moved:?}"
    );
    let up = frame(&ctx, vec![button(away, false)]);
    assert!(up.iter().any(|e| matches!(e, Input::Up(..))), "{up:?}");
}

#[test]
fn a_pointer_that_only_hovers_never_presses() {
    let ctx = Context::default();
    frame(&ctx, vec![Event::PointerMoved(pos2(100.0, 100.0))]);
    let hovering = frame(&ctx, vec![Event::PointerMoved(pos2(140.0, 120.0))]);
    assert_eq!(
        hovering,
        vec![Input::Move(pos2(140.0, 120.0), Mods::default())]
    );
}
