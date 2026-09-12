use eframe::egui::{Event, PointerButton, RawInput, Rect, TextureId, Vec2, pos2, vec2};

use super::*;

/// The canvas fills the whole of a window this size, in points.
const SCREEN: Vec2 = vec2(800.0, 600.0);

/// Anywhere well inside the canvas.
const OVER: egui::Pos2 = egui::pos2(400.0, 300.0);

/// Runs `steer` over one frame of raw input and hands back what the camera
/// projects afterwards.
///
/// Three passes, because egui hit-tests against the rect the last one
/// allocated: the image has to have been laid out before it can be dragged.
fn steered(texture: Option<TextureId>, frames: Vec<Vec<Event>>) -> [f32; 16] {
    let ctx = egui::Context::default();
    let theme = Theme::sastreria();
    theme.apply(&ctx);
    let mut camera = Camera::for_body();
    for events in frames {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), SCREEN)),
            events,
            ..Default::default()
        };
        let pass = ctx.run_ui(input, |ui| {
            steer(ui, &theme, SCREEN, texture, &mut camera);
        });
        pass.drop_without_applying_deltas();
    }
    camera.mvp(SCREEN.x / SCREEN.y)
}

/// Nothing happened, so nothing moved: the baseline the others are read
/// against.
fn still() -> [f32; 16] {
    steered(Some(TextureId::User(0)), vec![vec![]; 6])
}

/// How far two projections stand apart, summed over the matrix.
///
/// A sum and not an equality, so what the tests below assert is that the
/// camera moved by an amount a person would see, rather than that two floats
/// happen to differ in their last bit.
fn apart(a: [f32; 16], b: [f32; 16]) -> f32 {
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum()
}

fn press(at: egui::Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    }
}

#[test]
fn a_drag_across_the_canvas_orbits_the_camera() {
    let to = OVER + vec2(120.0, 40.0);
    let dragged = steered(
        Some(TextureId::User(0)),
        vec![
            vec![Event::PointerMoved(OVER)],
            vec![press(OVER, true)],
            vec![Event::PointerMoved(OVER + vec2(60.0, 20.0))],
            vec![Event::PointerMoved(to)],
            vec![press(to, false)],
            vec![],
        ],
    );
    let moved = apart(dragged, still());
    assert!(moved > 0.1, "the drag never reached the camera: {moved}");
}

#[test]
fn the_wheel_over_the_canvas_zooms_it() {
    let zoomed = steered(
        Some(TextureId::User(0)),
        vec![
            vec![Event::PointerMoved(OVER)],
            vec![Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0.0, -240.0),
                modifiers: egui::Modifiers::default(),
                phase: egui::TouchPhase::Move,
            }],
            vec![],
        ],
    );
    let moved = apart(zoomed, still());
    assert!(moved > 0.1, "the wheel never reached the camera: {moved}");
}

#[test]
fn a_canvas_with_nothing_painted_yet_still_takes_its_room() {
    // Otherwise the panels beside it jump on the frame the first paint lands.
    let ctx = egui::Context::default();
    let theme = Theme::sastreria();
    let mut camera = Camera::for_body();
    let mut taken = Vec2::ZERO;
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), SCREEN)),
        ..Default::default()
    };
    ctx.run_ui(input, |ui| {
        let before = ui.cursor().min;
        steer(ui, &theme, SCREEN, None, &mut camera);
        taken = ui.min_rect().max - before;
    })
    .drop_without_applying_deltas();
    assert!(taken.x >= SCREEN.x && taken.y >= SCREEN.y, "{taken:?}");
}
