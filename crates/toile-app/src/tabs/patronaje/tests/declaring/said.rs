use toile_engine::draft::PointKey;

use super::super::super::report::status;
use super::super::studio::Studio;
use super::drafted;

/// The cells of the bar right now, without whether each is an alert.
fn bar(studio: &Studio) -> Vec<String> {
    status(&studio.session, &studio.state)
        .into_iter()
        .map(|(cell, _)| cell)
        .collect()
}

/// Whether the bar says `said` in one of its cells.
fn says(studio: &Studio, said: &str) -> bool {
    bar(studio).iter().any(|cell| cell == said)
}

/// With the tool in hand and nothing pressed, the bar names both of the places
/// a press can go — which is the one thing that decides which of the tool's two
/// gestures opens.
#[test]
fn the_bar_names_both_of_the_tools_targets_before_a_press() {
    let (studio, _, _) = drafted();
    assert!(
        says(
            &studio,
            "pinza: pulsa un tramo recto para cortarla, o un nodo para declararla"
        ),
        "{:?}",
        bar(&studio)
    );
}

/// Between the presses the bar counts nodes and says «declarando», which is how
/// a person reads which of the two gestures they are in the middle of.
#[test]
fn between_the_presses_the_bar_counts_nodes_and_says_declaring() {
    let (mut studio, _, nodes) = drafted();
    let press = |studio: &mut Studio, node: PointKey| {
        let at = studio.on_glass(node);
        studio.click(at);
        studio.frame(Vec::new());
    };

    press(&mut studio, nodes[0]);
    assert!(
        says(&studio, "declarando pinza · 1 de 3 nodos"),
        "{:?}",
        bar(&studio)
    );
    assert!(says(&studio, "pulsa el pico: el nodo de al lado"));

    press(&mut studio, nodes[1]);
    assert!(
        says(&studio, "declarando pinza · 2 de 3 nodos"),
        "{:?}",
        bar(&studio)
    );
    assert!(says(
        &studio,
        "pulsa la segunda pata: el nodo que sigue al pico"
    ));
    assert!(
        says(
            &studio,
            "se plancha hacia la primera pata · Retroceso quita el último · Esc cancela"
        ),
        "{:?}",
        bar(&studio)
    );

    press(&mut studio, nodes[2]);
    assert!(
        !says(&studio, "declarando pinza · 3 de 3 nodos"),
        "the third press finishes it rather than counting it: {:?}",
        bar(&studio)
    );
    assert!(
        says(&studio, "deshacer declarar pinza"),
        "{:?}",
        bar(&studio)
    );
}

/// Backspace takes the last node back, and the bar counts down with it; a
/// second one leaves the gesture with nothing to unwind.
#[test]
fn backspace_takes_the_last_node_back_and_the_bar_counts_down() {
    let (mut studio, _, nodes) = drafted();
    for node in [nodes[0], nodes[1]] {
        let at = studio.on_glass(node);
        studio.click(at);
    }
    studio.frame(Vec::new());
    assert!(says(&studio, "declarando pinza · 2 de 3 nodos"));

    studio.key(eframe::egui::Key::Backspace, eframe::egui::Modifiers::NONE);
    studio.frame(Vec::new());
    assert!(
        says(&studio, "declarando pinza · 1 de 3 nodos"),
        "{:?}",
        bar(&studio)
    );

    studio.key(eframe::egui::Key::Backspace, eframe::egui::Modifiers::NONE);
    studio.frame(Vec::new());
    assert_eq!(studio.session.undo_label(), None, "nothing to unwind");
    assert!(studio.doc().darts.is_empty());
}
