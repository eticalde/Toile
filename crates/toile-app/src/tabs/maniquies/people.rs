mod own;
mod rows;

use eframe::egui;
use toile_engine::draft::PersonaError;
use toile_engine::session::Session;

use super::identity::note;
use super::stand::{Saved, Stand, Wrote};
use crate::band::Band;
use crate::config::Prefs;
use crate::library::shelf::Shelf;
use crate::library::today::today;
use crate::library::{LibraryError, Listed};
use crate::tabs::Kept;
use crate::theme::Theme;
use crate::widgets::{PAD, button_secondary, section_with};

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
    /// Make the person filed under this stem the one new products are cut
    /// for, or stop, when she already is.
    ToggleDefault(String),
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
///
/// `own` is the stem new products are cut for, from the preferences. Its row
/// says so, and stays listed, marked gone, once the file has left the
/// library, so the preference can still be seen and let go of.
pub fn panel(
    ui: &mut egui::Ui,
    theme: &Theme,
    shelf: &Shelf,
    people: &mut People,
    kept: Kept,
    own: Option<&str>,
) -> Option<Plea> {
    let listed = shelf.listed();
    let count = listed.map_or_else(|_| String::new(), |all| all.len().to_string());
    section_with(ui, theme, "Biblioteca", &count);
    if shelf.library().is_none() {
        note(ui, theme.muted, NO_FOLDER);
        own::line(ui, theme, shelf, own);
        return None;
    }
    let all = match listed {
        Err(why) => {
            let why = format!("no se pudo leer la biblioteca: {why}");
            note(ui, theme.alert, &why);
            &[]
        }
        Ok([]) => {
            note(ui, theme.muted, EMPTY);
            &[]
        }
        Ok(all) => all,
    };
    let gone = own.filter(|stem| listed.is_ok() && !all.iter().any(|each| each.stem == *stem));
    rows::list(ui, theme, (all, gone), people, own);
    own::line(ui, theme, shelf, own);
    let mut plea = actions(ui, theme, all, people, kept, own);
    ui.add_space(4.0);
    if button(ui, theme, "Guardar en biblioteca") {
        plea = Some(Plea::Save);
    }
    if let Some((text, bad)) = people.said() {
        note(ui, if bad { theme.alert } else { theme.muted }, text);
    }
    plea
}

/// What the picked row offers: a copy of the person, and the choice of her as
/// the one new products are cut for.
///
/// A file that does not read, or a default person gone from the library, can
/// no longer be used, and is offered only the way out of being the default:
/// a person nobody can copy could never be made it.
fn actions(
    ui: &mut egui::Ui,
    theme: &Theme,
    all: &[Listed],
    people: &People,
    kept: Kept,
    own: Option<&str>,
) -> Option<Plea> {
    let stem = people.picked.as_deref()?;
    let is_own = own == Some(stem);
    let found = all.iter().find(|each| each.stem == stem);
    let mut plea = None;
    let usable = match found.map(|listed| &listed.persona) {
        Some(Ok(_)) => {
            if button(ui, theme, use_label(kept)) {
                plea = Some(Plea::Use(stem.to_owned()));
            }
            true
        }
        Some(Err(why)) => {
            note(ui, theme.alert, &why.to_string());
            false
        }
        None => false,
    };
    if (usable || is_own) && own::toggle(ui, theme, is_own) {
        plea = Some(Plea::ToggleDefault(stem.to_owned()));
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
    prefs: &mut Prefs,
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
        Plea::ToggleDefault(stem) => own::choose(prefs, &stem),
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
