use eframe::egui::epaint::Shape;
use eframe::egui::{Color32, Event, Key, Modifiers, Pos2, pos2, vec2};
use toile_engine::draft::block;

use super::super::state::Scope;
use super::bench::front_and_back;
use super::button;
use super::studio::{BACK_ROW, FRONT_ROW, PLUS_ROW, Studio, WHOLE_ROW, painted};

/// Whether the last frame lit the spot `at` the way a row lights under the
/// pointer.
fn lit_under(studio: &Studio, at: Pos2) -> bool {
    let tint = studio.theme.accent.gamma_multiply(0.07);
    painted(studio)
        .iter()
        .any(|shape| matches!(shape, Shape::Rect(r) if r.fill == tint && r.rect.contains(at)))
}

/// The ink the last frame wrote `label` in, when it wrote it.
fn ink_of(studio: &Studio, label: &str) -> Option<Color32> {
    painted(studio).iter().find_map(|shape| match shape {
        Shape::Text(text) if text.galley.job.text == label => {
            text.galley.job.sections.first().map(|at| at.format.color)
        }
        _ => None,
    })
}

/// Whether anything the last frame drew under `at` would take a click.
fn takes_a_click(studio: &Studio, at: Pos2) -> bool {
    let Some(layer) = studio.ctx.layer_id_at(at) else {
        return false;
    };
    studio.ctx.viewport(|port| {
        port.prev_pass
            .widgets
            .get_layer(layer)
            .any(|widget| widget.interact_rect.contains(at) && widget.sense.senses_click())
    })
}

/// While the question a drag over a formula asks is up, the tree reads as
/// unavailable, the way the trail over the mat does: no row lights under the
/// pointer, every label is muted, nothing takes a click. Answered, it is live
/// again.
#[test]
fn while_a_question_waits_the_tree_looks_dead_and_is_dead() {
    let mut studio = Studio::new(block::trousers());
    let (front, _) = front_and_back(studio.doc());
    let hip = studio
        .doc()
        .shows_label(front, "cadera_lat")
        .expect("the block names the hip");
    studio.state.open(front);
    studio.frame(Vec::new());
    let grab = studio.on_glass(hip);
    studio.frame(vec![Event::PointerMoved(grab)]);
    studio.frame(vec![button(grab, true)]);
    for step in 1..=3 {
        let at = grab + vec2(30.0 * step as f32, 0.0);
        studio.frame(vec![Event::PointerMoved(at)]);
    }
    studio.frame(vec![button(grab + vec2(90.0, 0.0), false)]);
    assert!(studio.state.ask.is_some(), "a drag over a formula asks");

    let rows = [
        (WHOLE_ROW, "Todas las piezas"),
        (FRONT_ROW, block::FRONT),
        (BACK_ROW, block::BACK),
        (PLUS_ROW, "Pieza"),
    ];
    for (at, label) in rows {
        studio.frame(vec![Event::PointerMoved(at)]);
        assert!(!lit_under(&studio, at), "{label} lights under the pointer");
        assert!(!takes_a_click(&studio, at), "{label} takes a click");
        let muted = Some(studio.theme.muted);
        assert_eq!(ink_of(&studio, label), muted, "{label} is not muted");
    }
    // Where the pencil and the cross of a hovered row would be.
    for x in [180.0, 200.0, 220.0] {
        let at = pos2(x, FRONT_ROW.y);
        studio.frame(vec![Event::PointerMoved(at)]);
        assert!(!takes_a_click(&studio, at), "the icons at {x} take a click");
    }
    studio.click(WHOLE_ROW);
    assert_eq!(studio.state.scope, Scope::Piece, "the press went nowhere");
    assert!(studio.state.ask.is_some(), "and the question is still up");

    studio.key(Key::Enter, Modifiers::NONE);
    assert_eq!(studio.state.ask, None, "the question is answered");
    studio.frame(vec![Event::PointerMoved(BACK_ROW)]);
    assert!(lit_under(&studio, BACK_ROW), "a live row lights again");
    assert!(takes_a_click(&studio, BACK_ROW), "and takes a click");
    let soft = Some(studio.theme.ink_soft);
    assert_eq!(ink_of(&studio, block::BACK), soft);
    studio.click(WHOLE_ROW);
    assert_eq!(studio.state.scope, Scope::Product, "the tree leads again");
}
