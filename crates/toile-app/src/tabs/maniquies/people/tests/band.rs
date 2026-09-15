use eframe::egui::containers::panel::PanelState;
use eframe::egui::{self, Id, Rect, pos2, vec2};

use super::{ANA, Table, resolved, written};
use crate::band;
use crate::theme::Theme;

/// Ana used in a product that is then saved, and her waist corrected by hand
/// in her library file afterwards. Answers the table with the product opened
/// again, and the bytes it was saved with.
fn corrected(test: &str) -> (Table, String) {
    let mut table = Table::with_ana(test);
    table.use_ana();
    let saved = written(&table.session);
    let file = String::from_utf8(table.scratch.bytes(ANA)).expect("a person's file is text");
    let waist = "\"cintura\": 72";
    assert!(file.contains(waist), "{file}");
    let file = file.replacen(waist, "\"cintura\": 70", 1);
    table.scratch.put("ana.toile-persona", &file);
    table.reopen();
    (table, saved)
}

#[test]
fn a_measure_edited_in_the_library_file_raises_the_band_when_the_product_opens() {
    let (table, saved) = corrected("band");
    let offers = table.band.offers();
    assert_eq!(offers.len(), 1);
    assert_eq!(offers[0].name, "Ana");
    assert_eq!(table.session.revision(), 0, "opening edits nothing");
    assert_eq!(
        written(&table.session),
        saved,
        "opening never changes bytes"
    );
    assert!(!table.session.can_undo());
}

#[test]
fn a_person_the_library_no_longer_holds_raises_no_band() {
    let (mut table, _) = corrected("gone");
    std::fs::remove_file(table.scratch.file(ANA)).expect("Ana's file is removed");
    table.reopen();
    assert!(table.band.offers().is_empty());
}

/// Actualizar is an ordinary edit: the library's tape comes in with a link to
/// it, and one undo gives the product its bytes back.
#[test]
fn actualizar_brings_in_the_library_tape_and_one_undo_gives_the_bytes_back() {
    let (mut table, saved) = corrected("update");
    table.band.update(&mut table.session, 0);
    assert!(table.band.offers().is_empty());
    assert_eq!(resolved(&table.session, "cintura"), Some(70.0));
    assert_eq!(table.session.undo_label(), Some("actualizar maniquí"));
    let current = table.shelf.persona(ANA).and_then(|ana| ana.current());
    let linked = table.stand.body(&table.session).origin.as_ref();
    assert_eq!(
        linked.map(|origin| &origin.fnv),
        current
            .map(toile_engine::draft::Snapshot::fingerprint)
            .as_ref()
    );

    table.session.undo().expect("the update undoes");
    assert_eq!(written(&table.session), saved);
    assert!(!table.session.can_undo());
}

/// Mantener writes nothing, and the body stays declined while the product is
/// open; opening the product again asks again.
#[test]
fn mantener_writes_nothing_and_the_offer_stays_declined_while_the_product_is_open() {
    let (mut table, saved) = corrected("keep");
    table.band.keep(0);
    assert!(table.band.offers().is_empty());
    assert_eq!(table.session.revision(), 0);
    assert_eq!(written(&table.session), saved);

    table.band.recheck(&table.shelf, &table.session);
    assert!(
        table.band.offers().is_empty(),
        "declined until another opening"
    );
    table.reopen();
    assert_eq!(table.band.offers().len(), 1, "a new opening asks again");
}

/// An update the pattern cannot resolve with is refused whole: the offer
/// stands, the reason is shown, and the product keeps its bytes.
#[test]
fn an_update_the_pattern_cannot_resolve_with_is_refused_and_said() {
    let (mut table, saved) = corrected("refused");
    let mut ana = table.scratch.library().load(ANA).expect("Ana loads");
    let current = ana.taken.last_mut().expect("Ana has a session");
    current.values.remove("cadera");
    let file = ana.to_canonical_json().expect("writable");
    table.scratch.put("ana.toile-persona", &file);
    table.reopen();

    table.band.update(&mut table.session, 0);
    assert_eq!(table.band.offers().len(), 1, "the offer stands");
    let why = table.band.refused().unwrap_or_default();
    assert!(why.starts_with("no se pudo actualizar «Ana»"), "{why}");
    assert_eq!(written(&table.session), saved);
    assert!(!table.session.can_undo());
}

/// The band is a panel of the window, drawn before any tab, and only while an
/// offer stands.
#[test]
fn the_band_is_drawn_only_while_an_offer_stands() {
    let (mut table, _) = corrected("drawn");
    let drawn = |table: &mut Table| {
        let ctx = egui::Context::default();
        let theme = Theme::sastreria();
        theme.apply(&ctx);
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1320.0, 780.0));
        for _ in 0..2 {
            let input = egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            };
            let pass = ctx.run_ui(input, |ui| {
                band::show(ui, &theme, &mut table.band, &mut table.session);
            });
            pass.drop_without_applying_deltas();
        }
        PanelState::load(&ctx, Id::new("banda")).is_some()
    };
    assert!(drawn(&mut table));
    table.band.keep(0);
    assert!(!drawn(&mut table));
}
