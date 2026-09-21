use eframe::egui::{self, Id};

use super::People;
use super::own::{GONE_NOTE, ROW_NOTE};
use crate::library::shelf::shown;
use crate::library::{Listed, PERSONA_EXT};
use crate::theme::Theme;
use crate::widgets::list_row_named;

/// How many rows the list shows before it scrolls, so a long library cannot
/// push the buttons out of the panel.
const ROWS: f32 = 6.0;

/// The height of one row, as the panel widgets lay it out.
const ROW_H: f32 = 26.0;

/// The rows, scrolling past a few, then the row of a default person whose file
/// is `gone`; a press picks the row. Nothing at all when there is nobody to
/// list.
pub(super) fn list(
    ui: &mut egui::Ui,
    theme: &Theme,
    (all, gone): (&[Listed], Option<&str>),
    people: &mut People,
    own: Option<&str>,
) {
    if all.is_empty() && gone.is_none() {
        return;
    }
    egui::ScrollArea::vertical()
        .id_salt("biblioteca")
        .max_height(ROW_H * ROWS)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for listed in all {
                let stem = listed.stem.as_str();
                let picked = people.picked.as_deref() == Some(stem);
                if row(ui, theme, listed, picked, own == Some(stem)).clicked() {
                    people.picked = Some(listed.stem.clone());
                }
            }
            if let Some(stem) = gone {
                let picked = people.picked.as_deref() == Some(stem);
                let note = (GONE_NOTE, theme.alert);
                if list_row_named(ui, theme, row_id(stem), (stem, picked), note).clicked() {
                    people.picked = Some(stem.to_owned());
                }
            }
        });
}

/// The identity the row of the person filed under `stem` is drawn under, so a
/// press aimed from outside the panel finds her row by whom it lists.
pub(super) fn row_id(stem: &str) -> Id {
    Id::new(("biblioteca", stem))
}

/// One file of the library: the person's name and the day of her current
/// session, or that new products are cut for her; or the file's own name when
/// it does not read as anyone.
fn row(
    ui: &mut egui::Ui,
    theme: &Theme,
    listed: &Listed,
    picked: bool,
    own: bool,
) -> egui::Response {
    let id = row_id(&listed.stem);
    let Ok(persona) = &listed.persona else {
        let file = format!("{}.{PERSONA_EXT}", listed.stem);
        return list_row_named(ui, theme, id, (&file, picked), ("ilegible", theme.alert));
    };
    let day = persona
        .current()
        .map_or("", |current| current.date.as_str());
    let note = if own {
        (ROW_NOTE, theme.accent)
    } else {
        (day, theme.muted)
    };
    let name = shown(&listed.stem, persona);
    list_row_named(ui, theme, id, (name, picked), note)
}
