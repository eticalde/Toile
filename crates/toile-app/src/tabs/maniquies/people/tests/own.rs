use std::path::Path;

use eframe::egui::{self, Event, Id, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};
use toile_engine::draft::Persona;

use super::super::own::toggle_id;
use super::super::panel;
use super::super::rows::row_id;
use super::{ANA, Table, ana};
use crate::config::Prefs;
use crate::tabs::{Kept, left_panel};
use crate::theme::Theme;

/// A context dressed the way the app dresses its own.
fn screen() -> egui::Context {
    let ctx = egui::Context::default();
    Theme::sastreria().apply(&ctx);
    ctx
}

/// One frame of the library section fed `events`, and whatever it asked for
/// done, the way the tab does it.
fn frame(table: &mut Table, ctx: &egui::Context, events: Vec<Event>) {
    let theme = Theme::sastreria();
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1320.0, 780.0))),
        events,
        ..RawInput::default()
    };
    let own = table.prefs.default_persona.clone();
    let mut asked = None;
    let pass = ctx.run_ui(input, |ui| {
        asked = left_panel(ui, &theme, |ui| {
            let (shelf, people) = (&table.shelf, &mut table.people);
            panel(ui, &theme, shelf, people, Kept::InProduct, own.as_deref())
        });
    });
    pass.drop_without_applying_deltas();
    if let Some(plea) = asked {
        table.act(plea);
    }
}

/// A few frames with nobody touching anything, so every rect is laid out.
fn settle(table: &mut Table, ctx: &egui::Context) {
    for _ in 0..3 {
        frame(table, ctx, Vec::new());
    }
}

/// The pointer arriving on what the last frame drew under `id`, pressing and
/// letting go, a frame each.
fn click(table: &mut Table, ctx: &egui::Context, id: Id) {
    let at = ctx
        .read_response(id)
        .expect("the panel drew it")
        .rect
        .center();
    frame(table, ctx, vec![Event::PointerMoved(at)]);
    for pressed in [true, false] {
        let button = PointerButton::Primary;
        let modifiers = Modifiers::NONE;
        let event = Event::PointerButton {
            pos: at,
            button,
            pressed,
            modifiers,
        };
        frame(table, ctx, vec![event]);
    }
}

/// Whether the last frame drew anything under `id`.
fn drew(ctx: &egui::Context, id: Id) -> bool {
    ctx.read_response(id).is_some()
}

/// The default person the preferences file on disk names.
///
/// # Panics
/// When there is no file, which only a press should have written.
fn on_disk(file: &Path) -> Option<String> {
    assert!(file.exists(), "a press writes the preferences");
    Prefs::load_from(file).default_persona
}

/// The toggle makes the picked person the default, moves it to the next one
/// picked, and lets it go when pressed on her again; each press writes the
/// preferences on the spot, and nothing else writes them.
#[test]
fn the_toggle_sets_moves_and_clears_the_default_and_only_a_press_writes() {
    let mut table = Table::with_ana("own-toggle");
    let bea = Persona {
        name: "Bea".to_owned(),
        ..ana(&table.session)
    };
    let bea = table.scratch.library().create(&bea).expect("Bea is filed");
    table.shelf.refresh();
    let file = table.scratch.beside("prefs.json");
    let ctx = screen();

    settle(&mut table, &ctx);
    assert!(!drew(&ctx, toggle_id()), "nobody picked, nothing offered");
    click(&mut table, &ctx, row_id(ANA));
    assert_eq!(table.people.picked.as_deref(), Some(ANA));
    assert!(!file.exists(), "drawing and picking write nothing");

    click(&mut table, &ctx, toggle_id());
    assert_eq!(table.prefs.default_persona.as_deref(), Some(ANA));
    assert_eq!(on_disk(&file).as_deref(), Some(ANA));

    click(&mut table, &ctx, row_id(&bea));
    click(&mut table, &ctx, toggle_id());
    assert_eq!(table.prefs.default_persona.as_deref(), Some(bea.as_str()));
    assert_eq!(on_disk(&file), Some(bea.clone()), "it moved");

    click(&mut table, &ctx, toggle_id());
    assert_eq!(table.prefs.default_persona, None);
    assert_eq!(on_disk(&file), None, "the same choice again clears it");
    let text = std::fs::read_to_string(&file).expect("the file is there");
    assert!(!text.contains("default_persona"), "{text}");
    settle(&mut table, &ctx);
    assert_eq!(
        table.prefs.default_persona, None,
        "frames alone move nothing"
    );
}

/// A default person whose file has left the library keeps a row, so the
/// preference is still seen, and can be let go of there.
#[test]
fn a_default_person_gone_from_the_library_keeps_a_row_and_can_be_let_go() {
    let mut table = Table::with_ana("own-gone");
    table.prefs.toggle_default(ANA);
    std::fs::remove_file(table.scratch.file(ANA)).expect("Ana's file is removed");
    table.shelf.refresh();
    let ctx = screen();

    settle(&mut table, &ctx);
    assert!(drew(&ctx, row_id(ANA)), "her row stays, marked gone");
    click(&mut table, &ctx, row_id(ANA));
    click(&mut table, &ctx, toggle_id());
    assert_eq!(table.prefs.default_persona, None);
    settle(&mut table, &ctx);
    assert!(!drew(&ctx, row_id(ANA)), "once let go, nobody is listed");
}

/// A file that does not read can never be made the default, but a default
/// whose file stopped reading can still be let go of.
#[test]
fn an_unreadable_file_is_offered_only_the_way_out_of_being_the_default() {
    let mut table = Table::with_ana("own-unreadable");
    table
        .scratch
        .put("rota.toile-persona", "{\"toile_persona\": 1,");
    table.shelf.refresh();
    let ctx = screen();

    settle(&mut table, &ctx);
    click(&mut table, &ctx, row_id("rota"));
    assert_eq!(table.people.picked.as_deref(), Some("rota"));
    assert!(
        !drew(&ctx, toggle_id()),
        "not offered to make it the default"
    );

    table.prefs.toggle_default("rota");
    settle(&mut table, &ctx);
    click(&mut table, &ctx, toggle_id());
    assert_eq!(table.prefs.default_persona, None);
}
