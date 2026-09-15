use eframe::egui;
use toile_engine::draft::PersonaError;
use toile_engine::session::Session;

use super::identity::note;
use super::stand::{Saved, Stand, Wrote};
use crate::band::Band;
use crate::library::shelf::Shelf;
use crate::library::today::today;
use crate::library::{LibraryError, Listed, PERSONA_EXT};
use crate::tabs::Kept;
use crate::theme::Theme;
use crate::widgets::{PAD, button_secondary, list_row_noted, section_with};

/// How many rows the list shows before it scrolls, so a long library cannot
/// push the two buttons out of the panel.
const ROWS: f32 = 6.0;

/// The height of one row, as the panel widgets lay it out.
const ROW_H: f32 = 26.0;

/// What the section says of a library nobody has been saved to.
const EMPTY: &str = "Aún no hay nadie en la biblioteca. «Guardar en biblioteca» guarda aquí el \
                     maniquí de la mesa, con la fecha de hoy.";

/// What the section says where the system names no data folder.
const NO_FOLDER: &str = "El sistema no indica una carpeta de datos, así que no hay biblioteca \
                         donde guardar personas.";

/// What the library section asks the tab to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plea {
    /// Put a copy of the person filed under this stem on the stand.
    Use(String),
    /// Save the body on the stand to the library.
    Save,
}

/// What the library section keeps between frames.
#[derive(Debug, Default)]
pub struct People {
    /// The stem of the row picked in the list.
    pub picked: Option<String>,
    /// What the last save had to say, and whether it is that it failed.
    said: Option<(String, bool)>,
}

impl People {
    /// What the last save had to say, and whether it is that it failed.
    pub fn said(&self) -> Option<(&str, bool)> {
        self.said.as_ref().map(|(text, bad)| (text.as_str(), *bad))
    }

    /// Forgets what was said about the product that was open.
    pub fn forget(&mut self) {
        self.said = None;
    }
}

/// The people in the library, under the body's own controls: a row per file
/// with the day of the person's current session, then the two ways a body
/// crosses between the library and the stand.
pub fn panel(
    ui: &mut egui::Ui,
    theme: &Theme,
    shelf: &Shelf,
    people: &mut People,
    kept: Kept,
) -> Option<Plea> {
    let listed = shelf.listed();
    let count = listed.map_or_else(|_| String::new(), |all| all.len().to_string());
    section_with(ui, theme, "Biblioteca", &count);
    if shelf.library().is_none() {
        note(ui, theme.muted, NO_FOLDER);
        return None;
    }
    let mut plea = None;
    match listed {
        Err(why) => note(
            ui,
            theme.alert,
            &format!("no se pudo leer la biblioteca: {why}"),
        ),
        Ok([]) => note(ui, theme.muted, EMPTY),
        Ok(all) => {
            egui::ScrollArea::vertical()
                .id_salt("biblioteca")
                .max_height(ROW_H * ROWS)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for listed in all {
                        let picked = people.picked.as_deref() == Some(listed.stem.as_str());
                        if row(ui, theme, listed, picked).clicked() {
                            people.picked = Some(listed.stem.clone());
                        }
                    }
                });
        }
    }
    let picked = listed.ok().and_then(|all| {
        all.iter()
            .find(|each| people.picked.as_ref() == Some(&each.stem))
    });
    match picked.map(|listed| (&listed.stem, &listed.persona)) {
        Some((stem, Ok(_))) => {
            if button(ui, theme, use_label(kept)) {
                plea = Some(Plea::Use(stem.clone()));
            }
        }
        Some((_, Err(why))) => note(ui, theme.alert, &why.to_string()),
        None => {}
    }
    ui.add_space(4.0);
    if button(ui, theme, "Guardar en biblioteca") {
        plea = Some(Plea::Save);
    }
    if let Some((text, bad)) = people.said() {
        note(ui, if bad { theme.alert } else { theme.muted }, text);
    }
    plea
}

/// Does what the library section asked, and reads the library again after
/// writing to it, so the list and the band see what was written.
pub fn act(
    plea: Plea,
    session: &mut Session,
    stand: &mut Stand,
    shelf: &mut Shelf,
    band: &mut Band,
    people: &mut People,
) {
    people.said = None;
    match plea {
        Plea::Use(stem) => {
            if let Some(persona) = shelf.persona(&stem) {
                stand.use_persona(session, &stem, persona);
            } else {
                let gone = format!("«{stem}» ya no está en la biblioteca");
                people.said = Some((gone, true));
            }
        }
        Plea::Save => {
            let Some(library) = shelf.library() else {
                return;
            };
            people.said = Some(match stand.save_to(session, library, today()) {
                Ok(saved) => (saved_text(&saved), false),
                Err(why) => (
                    format!("no se pudo guardar en la biblioteca: {}", told(&why)),
                    true,
                ),
            });
            shelf.refresh();
            band.recheck(shelf, session);
        }
    }
}

/// One file of the library: the person's name and the day of her current
/// session, or the file's own name when it does not read as anyone.
fn row(ui: &mut egui::Ui, theme: &Theme, listed: &Listed, picked: bool) -> egui::Response {
    let Ok(persona) = &listed.persona else {
        let file = format!("{}.{PERSONA_EXT}", listed.stem);
        return list_row_noted(ui, theme, &file, picked, ("ilegible", theme.alert));
    };
    let name = if persona.name.trim().is_empty() {
        &listed.stem
    } else {
        &persona.name
    };
    let day = persona
        .current()
        .map_or("", |current| current.date.as_str());
    list_row_noted(ui, theme, name, picked, (day, theme.muted))
}

/// A panel-wide action button, answering whether it was pressed.
fn button(ui: &mut egui::Ui, theme: &Theme, label: &str) -> bool {
    ui.horizontal(|ui| {
        ui.add_space(PAD);
        button_secondary(ui, theme, label).clicked()
    })
    .inner
}

/// What using a person does, said on the button that does it.
fn use_label(kept: Kept) -> &'static str {
    match kept {
        Kept::InProduct => "Usar en el producto",
        Kept::Nowhere => "Usar en la mesa",
    }
}

fn saved_text(saved: &Saved) -> String {
    let Saved { name, date, wrote } = saved;
    match wrote {
        Wrote::Created => format!("«{name}» entra en la biblioteca · {date}"),
        Wrote::Recorded => format!("nueva sesión de «{name}» · {date}"),
        Wrote::Nothing => format!("«{name}» ya tenía esta sesión · {date}"),
    }
}

/// A library failure in the words the panel shows.
///
/// The one refusal a save from here can meet is a person whose file holds a
/// session dated after today, and the format's own words for it are not the
/// interface's.
fn told(why: &LibraryError) -> String {
    match why {
        LibraryError::Refused(PersonaError::OutOfOrder { after, .. }) => {
            format!("la biblioteca ya tiene una sesión del {after}, posterior a hoy")
        }
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests;
