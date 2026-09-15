use eframe::egui::{self, Rect, pos2, vec2};
use toile_engine::session::Session;

use super::super::measures;
use super::super::stand::Stand;
use crate::tabs::right_panel;
use crate::theme::Theme;

/// The measures panel drawn for a few frames with the pointer held at `at`,
/// answering which row's tape the stand ends up wanting on the body.
///
/// Several frames because a hover is decided against the rects the previous
/// frame left behind, so the first one lands on nothing.
fn hovering(session: &mut Session, stand: &mut Stand, at: egui::Pos2) -> Option<String> {
    let ctx = egui::Context::default();
    let theme = Theme::sastreria();
    theme.apply(&ctx);
    let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1320.0, 780.0));
    for _ in 0..3 {
        let input = egui::RawInput {
            screen_rect: Some(screen),
            events: vec![egui::Event::PointerMoved(at)],
            ..Default::default()
        };
        let pass = ctx.run_ui(input, |ui| {
            right_panel(ui, &theme, |ui| {
                measures::panel(ui, &theme, session, stand);
            });
        });
        pass.drop_without_applying_deltas();
    }
    stand.highlight.clone()
}

/// The tape follows the hand: a row under the pointer lays its tape, and the
/// pointer leaving the panel takes it off again rather than leaving the last
/// one stuck on the body.
#[test]
fn the_tape_leaves_the_body_when_the_pointer_leaves_the_measures() {
    let (mut session, mut stand) = super::product();

    // Which row sits at which height is layout, so the pointer walks down the
    // panel until it finds one rather than trusting a number.
    let mut lit = None;
    let mut found_at = 0.0;
    for step in 0u8..40 {
        let y = 60.0 + f32::from(step) * 14.0;
        if let Some(name) = hovering(&mut session, &mut stand, pos2(1200.0, y)) {
            lit = Some(name);
            found_at = y;
            break;
        }
    }
    let lit = lit.expect("some row of the panel lights under the pointer");
    assert!(
        toile_engine::draft::MeasureSet::is_catalogued(&lit),
        "{lit} is not a catalogue name"
    );

    // Still lit while the pointer stays on it, so the next assertion is about
    // the pointer leaving and not about a frame that forgot.
    assert_eq!(
        hovering(&mut session, &mut stand, pos2(1200.0, found_at)),
        Some(lit),
        "the row keeps its tape while the pointer is on it"
    );
    assert_eq!(
        hovering(&mut session, &mut stand, pos2(40.0, 400.0)),
        None,
        "the tape comes off once the pointer is away from the measures"
    );
}
