use eframe::egui::{self, Id};

use super::super::identity::note;
use crate::config::Prefs;
use crate::library::shelf::{Shelf, shown};
use crate::theme::Theme;
use crate::widgets::check_named;

/// What the toggle says. It acts for the person picked in the list.
const LABEL: &str = "Por defecto para productos nuevos";

/// What the row of the person new products are cut for says, where every
/// other row says the day of its current session.
pub const ROW_NOTE: &str = "por defecto";

/// What the row of a default person the library no longer holds says.
pub const GONE_NOTE: &str = "no está";

/// What a new product starts as when the person cannot be had.
const BLANK: &str = "los productos nuevos empiezan con un maniquí sin medidas.";

/// The identity the toggle is drawn under, so a press aimed from outside the
/// panel finds it by what it does.
pub fn toggle_id() -> Id {
    Id::new("por-defecto-para-productos-nuevos")
}

/// The toggle for the picked person, ticked while she is the one new products
/// are cut for. Answers whether it was pressed.
pub fn toggle(ui: &mut egui::Ui, theme: &Theme, on: bool) -> bool {
    check_named(ui, theme, toggle_id(), LABEL, on).clicked()
}

/// Whom new products are cut for, said under the list whenever the
/// preferences name anyone.
///
/// A new product whose person cannot be had falls back to a blank one without
/// a word, so this line is the one place that says so: in the ink of a fault,
/// with what a new product starts as instead.
pub fn line(ui: &mut egui::Ui, theme: &Theme, shelf: &Shelf, own: Option<&str>) {
    let Some(stem) = own else {
        return;
    };
    let (text, ink) = match whose(shelf, stem) {
        Ok(name) => (
            format!("Los productos nuevos se cortan para «{name}»."),
            theme.muted,
        ),
        Err(why) => (format!("{why}: {BLANK}"), theme.alert),
    };
    note(ui, ink, &text);
}

/// Makes `stem` the person new products are cut for, taking the place from
/// whoever had it, or lets it go when it was already hers; and writes the
/// preferences down on the spot, since a toggle is a decision and not a
/// window size to keep until exit.
pub fn choose(prefs: &mut Prefs, stem: &str) {
    prefs.toggle_default(stem);
    prefs.save();
}

/// The name of the person `stem` names, or why the library cannot hand her
/// over.
fn whose<'a>(shelf: &'a Shelf, stem: &'a str) -> Result<&'a str, String> {
    let listed = shelf.library().and_then(|_| shelf.listed().ok());
    let Some(all) = listed else {
        return Err(format!("no se puede leer a «{stem}» en la biblioteca"));
    };
    match all.iter().find(|listed| listed.stem == stem) {
        None => Err(format!("«{stem}» ya no está en la biblioteca")),
        Some(listed) => match &listed.persona {
            Ok(persona) => Ok(shown(stem, persona)),
            Err(_) => Err(format!("el archivo de «{stem}» no se puede leer")),
        },
    }
}
