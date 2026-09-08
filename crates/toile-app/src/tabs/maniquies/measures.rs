use eframe::egui::{self, Id};

use super::State;
use crate::theme::Theme;
use crate::widgets::{Editable, Edited, formula_row, section, section_with};

/// Girth measurements, each catalogue name with the label the panel shows,
/// ordered top-down the body: the tape starts at the neck and ends at the
/// ankle, then the arm and the head, which sit off the trunk's line.
const CONTORNOS: [(&str, &str); 12] = [
    ("cuello", "Cuello"),
    ("pecho_alto", "Pecho alto"),
    ("pecho", "Contorno de pecho"),
    ("bajo_pecho", "Bajo pecho"),
    ("cintura", "Cintura"),
    ("cadera", "Cadera"),
    ("muslo", "Muslo"),
    ("rodilla", "Rodilla"),
    ("tobillo", "Tobillo"),
    ("brazo_contorno", "Contorno de brazo"),
    ("muneca", "Muñeca"),
    ("cabeza", "Contorno de cabeza"),
];

/// Vertical measurements and the one width (the shoulders), top-down as
/// well: the trunk lengths, the arm, then the legs.
const LARGOS: [(&str, &str); 7] = [
    ("largo_espalda", "Largo de espalda"),
    ("brazo", "Largo de brazo"),
    ("hombros", "Ancho de hombros"),
    ("tiro", "Tiro"),
    ("largo_lateral", "Largo lateral"),
    ("entrepierna", "Entrepierna"),
    ("altura_cadera", "Altura de cadera"),
];

/// The one whole-body measurement.
const CUERPO: [(&str, &str); 1] = [("estatura", "Estatura")];

/// The editable inspector: every catalogue measurement over the line that says
/// what it comes to. A confirmed value goes to the measure set and marks the
/// state dirty, so the central panel rebuilds the body before the next frame.
///
/// Twenty rows outgrow any window height, so the groups scroll under a pinned
/// title; the sections keep their order, so the scroll position is the only
/// thing that moves.
pub fn panel(ui: &mut egui::Ui, theme: &Theme, st: &mut State) {
    section_with(ui, theme, "Medidas · Etienne", "cm");
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            section(ui, theme, "Contornos");
            for entry in CONTORNOS {
                row(ui, theme, st, entry);
            }
            section(ui, theme, "Largos y anchos");
            for entry in LARGOS {
                row(ui, theme, st, entry);
            }
            section(ui, theme, "Cuerpo");
            for entry in CUERPO {
                row(ui, theme, st, entry);
            }
        });
}

/// Draws one measurement row and, when it is confirmed with a number, writes it
/// back and asks for a rebuild.
///
/// The in-progress text lives on the tab, so a row with the focus paints
/// whatever has been typed — faults and all — while the measure set keeps the
/// last value that parsed. Nothing reaches the body until a finite number does.
fn row(ui: &mut egui::Ui, theme: &Theme, st: &mut State, entry: (&str, &str)) {
    let (name, label) = entry;
    // The state seeds every catalogue name from `default_measures`, so a row
    // always has a value; 0.0 only guards a name deliberately cleared.
    let value = st.measures.get(name).unwrap_or(0.0);
    let source = format!("{value:.1}");

    let held = st
        .editing
        .as_ref()
        .filter(|(of, _)| of == name)
        .map(|(_, text)| text.clone());
    let fault = held.as_deref().and_then(unparsed);
    let note = fault.clone().unwrap_or_else(|| "cm".to_owned());

    let edited = formula_row(
        ui,
        theme,
        Id::new(("maniquies-measure", name)),
        &Editable {
            label,
            source: &source,
            note: &note,
            fault: fault.is_some(),
            held: held.as_deref(),
        },
    );
    match edited {
        Edited::Idle => {}
        Edited::Typing(text) => st.editing = Some((name.to_owned(), text)),
        Edited::Done(text) => match text.trim().parse::<f64>() {
            Ok(to) if to.is_finite() => {
                st.measures.values.insert(name.to_owned(), to);
                st.dirty = true;
                st.editing = None;
                ui.ctx().request_repaint();
            }
            // Keep the unparsed text so the faulted row goes on showing it.
            _ => st.editing = Some((name.to_owned(), text)),
        },
    }
}

/// Why the text in a box is not a measurement, or `None` when it is a number.
fn unparsed(text: &str) -> Option<String> {
    match text.trim().parse::<f64>() {
        Ok(value) if value.is_finite() => None,
        _ => Some("no es un número".to_owned()),
    }
}
