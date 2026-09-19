use eframe::egui::{Rect, pos2, vec2};
use toile_engine::body::Collider;
use toile_engine::draft::block;

use super::*;

/// Paints one pass of whatever `add` draws, over a screen-sized window.
///
/// A panel here is a painter and a set of allocations, nothing else: no GPU,
/// no window, and no state of its own to seed.
fn paint(mut add: impl FnMut(&mut egui::Ui, &Theme)) {
    let ctx = egui::Context::default();
    let theme = Theme::sastreria();
    theme.apply(&ctx);
    let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1320.0, 780.0));
    let input = egui::RawInput {
        screen_rect: Some(screen),
        ..Default::default()
    };
    let pass = ctx.run_ui(input, |ui| add(ui, &theme));
    pass.drop_without_applying_deltas();
}

/// Paints the inspector over `session`.
fn panel(session: &Session) {
    paint(|ui, theme| {
        right_panel(ui, theme, |ui| inspector(ui, theme, session));
    });
}

/// The seam table draws over a table with nothing on it, and over the block.
///
/// The rows it draws come from the document, so the panel has to survive a
/// document that has no seams as readily as one that has two.
#[test]
fn the_seam_table_paints_with_and_without_a_document() {
    panel(&Session::blank(Collider::demo()));
    let session = Session::from_doc(block::trousers(), Collider::demo()).expect("the block opens");
    assert_eq!(
        seams::measured(session.draft().expect("a document")).len(),
        2
    );
    panel(&session);
}

/// The bar names the body the document resolves against, and never an empty
/// box: a product made from scratch carries a mannequin nobody has named.
#[test]
fn the_bar_names_the_body_even_when_the_document_has_not() {
    let blank = Session::blank(Collider::demo());
    assert_eq!(fitted(&blank), Some(UNNAMED));
    let session = Session::from_doc(block::trousers(), Collider::demo()).expect("the block opens");
    assert_eq!(fitted(&session), Some("Etienne"));

    paint(|ui, theme| sub_bar(ui, theme, &session, "en caché", None));
    paint(|ui, theme| sub_bar(ui, theme, &blank, "horneando…", Some("14 · entrepierna")));
}

/// The bar says where a body crosses itself when a garment can reach it, and
/// says nothing of the soles the body the app opens with crosses at.
#[test]
fn the_bar_names_a_crossing_a_garment_can_reach_and_keeps_quiet_about_soles() {
    use toile_engine::body::bake::crossings;
    use toile_engine::body::{self, NO_LEVERS, Phenotype, body_mesh};

    let solved = body::solve_anny(&body::default_measures(), &Phenotype::default());
    let opening = crossings(&body_mesh(&solved.phenotype, &solved.levers));
    assert!(!opening.all().is_empty(), "its soles do cross");
    assert_eq!(note::crossed(&opening), None);

    let heavy = Phenotype {
        gender: 0.0,
        weight: 1.0,
        muscle: 1.0,
        ..Phenotype::default()
    };
    let found = crossings(&body_mesh(&heavy, &NO_LEVERS));
    assert_eq!(note::crossed(&found).as_deref(), Some("14 · entrepierna"));
}
