use eframe::egui::{self, Rect, pos2, vec2};

use super::super::panel;
use super::{ANA, People, Plea, Scratch, Shelf, Table};
use crate::tabs::{Kept, left_panel};
use crate::theme::Theme;

/// The library section drawn for a few frames with nobody touching it, and
/// whatever it asked for.
fn look(shelf: &Shelf, people: &mut People) -> Option<Plea> {
    let ctx = egui::Context::default();
    let theme = Theme::sastreria();
    theme.apply(&ctx);
    let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1320.0, 780.0));
    let mut asked = None;
    for _ in 0..3 {
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        let pass = ctx.run_ui(input, |ui| {
            let plea = left_panel(ui, &theme, |ui| {
                panel(ui, &theme, shelf, people, Kept::InProduct)
            });
            asked = asked.take().or(plea);
        });
        pass.drop_without_applying_deltas();
    }
    asked
}

/// The list is read when the library can have changed, not on each frame it is
/// drawn; and a file that does not parse is listed, not hidden.
#[test]
fn the_list_is_read_again_after_a_change_and_not_on_every_frame() {
    let mut table = Table::with_ana("list");
    table
        .scratch
        .put("rota.toile-persona", "{\"toile_persona\": 1,");
    assert_eq!(look(&table.shelf, &mut table.people), None);
    let count = table.shelf.listed().map(<[_]>::len).ok();
    assert_eq!(count, Some(1), "drawing reads nothing");

    table.shelf.refresh();
    let listed = table.shelf.listed().expect("the folder reads");
    let files: Vec<(&str, bool)> = listed
        .iter()
        .map(|each| (each.stem.as_str(), each.persona.is_ok()))
        .collect();
    assert_eq!(files, [(ANA, true), ("rota", false)]);
    table.people.picked = Some("rota".to_owned());
    assert_eq!(look(&table.shelf, &mut table.people), None);
}

/// An empty library and a system with no data folder both draw, and neither
/// asks for anything on its own.
#[test]
fn an_empty_library_and_a_missing_one_draw_and_ask_nothing() {
    let scratch = Scratch::new("empty");
    let mut people = People::default();
    let empty = Shelf::over(Some(scratch.library()));
    assert_eq!(empty.listed().map(<[_]>::len).ok(), Some(0));
    assert_eq!(look(&empty, &mut people), None);
    assert_eq!(look(&Shelf::over(None), &mut people), None);
}
