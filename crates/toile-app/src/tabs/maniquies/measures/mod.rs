mod delta;

use eframe::egui::{self, RichText, Slider, SliderClamping};
use toile_engine::body::BodyModel;

use super::State;
use crate::theme::Theme;
use crate::widgets::{PAD, footer_note, section, section_with};

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

/// The width the slider leaves for its value box and the panel's padding.
const VALUE_W: f32 = 78.0;

/// The span a slider offers each measurement, in centimetres: wide enough for
/// real bodies and every factory size, tight enough that a drag stays fine.
/// A value typed into the box may sit outside it.
fn span(name: &str) -> (f64, f64) {
    match name {
        "estatura" => (140.0, 210.0),
        // A neck and a knee happen to span the same tape, as do a back
        // length and a shoulder width; the pairs are one arm each.
        "cuello" | "rodilla" => (28.0, 60.0),
        "pecho_alto" => (60.0, 150.0),
        "pecho" => (60.0, 160.0),
        "bajo_pecho" => (55.0, 140.0),
        "cintura" => (50.0, 150.0),
        "cadera" => (60.0, 170.0),
        "muslo" => (35.0, 90.0),
        "tobillo" => (16.0, 40.0),
        "brazo_contorno" => (18.0, 55.0),
        "muneca" => (12.0, 26.0),
        "cabeza" => (48.0, 66.0),
        "largo_espalda" | "hombros" => (30.0, 60.0),
        "brazo" => (45.0, 80.0),
        "tiro" => (18.0, 40.0),
        "largo_lateral" => (80.0, 130.0),
        "entrepierna" => (55.0, 100.0),
        "altura_cadera" => (12.0, 30.0),
        _ => (0.0, 250.0),
    }
}

/// The editable inspector: every catalogue measurement as a slider with its
/// value box. A change goes to the measure set and marks the state dirty, so
/// the central panel rebuilds the body before the next frame; a row under the
/// pointer or in hand lights its region on the body.
///
/// Twenty rows outgrow any window height, so the groups scroll under a pinned
/// title; the sections keep their order, so the scroll position is the only
/// thing that moves.
pub fn panel(ui: &mut egui::Ui, theme: &Theme, st: &mut State) {
    section_with(ui, theme, "Medidas · Etienne", "cm");
    if st.model == BodyModel::Anny {
        // Honest rather than mysterious: the phenotype shapes this body,
        // and every row below now says how far the body actually lands
        // from the tape — dado, medido, Δ. Making a value exact, so medido
        // catches up to dado, is the next slice's per-part solver.
        footer_note(
            ui,
            theme,
            "El fenotipo da forma a este cuerpo; cada fila dice cuánto mide de verdad y a qué \
             distancia queda de tu cinta. Ajustarlo hasta que coincidan es la siguiente pieza.",
        );
    }
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
            ui.add_space(PAD);
        });
}

/// One measurement: its label over a slider that fills the panel. The label
/// takes the accent while the row is the one lighting the body.
fn row(ui: &mut egui::Ui, theme: &Theme, st: &mut State, entry: (&str, &str)) {
    let (name, label) = entry;
    // The state seeds every catalogue name from `default_measures`, so a row
    // always has a value; 0.0 only guards a name deliberately cleared.
    let mut value = st.measures.get(name).unwrap_or(0.0);
    let (lo, hi) = span(name);
    let lit = st.highlight.as_deref() == Some(name);
    let ink = if lit { theme.accent } else { theme.ink_soft };

    let scoped = ui.scope(|ui| {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(PAD);
            ui.label(RichText::new(label).size(12.0).color(ink));
        });
        ui.horizontal(|ui| {
            ui.add_space(PAD);
            ui.spacing_mut().slider_width = (ui.available_width() - PAD - VALUE_W).max(60.0);
            ui.add(
                Slider::new(&mut value, lo..=hi)
                    .suffix(" cm")
                    .fixed_decimals(1)
                    .trailing_fill(true)
                    .clamping(SliderClamping::Edits),
            )
        })
        .inner
    });
    let slider = scoped.inner;

    if slider.changed() && value.is_finite() {
        st.measures.values.insert(name.to_owned(), value);
        ui.ctx().request_repaint();
    }
    // A rebuild — and, for Anny, a full 20-row solve — only runs once the
    // edit commits (the slider releases, a click lands, or the value box
    // loses focus), never once per drag frame: see `build`'s own doc.
    if (slider.drag_stopped() || slider.lost_focus() || slider.clicked()) && value.is_finite() {
        st.dirty = true;
        ui.ctx().request_repaint();
    }
    // Hovering anywhere on the row, or holding the slider, lights its region;
    // the last one lit stays so once the pointer moves on to the body.
    if scoped.response.hovered() || slider.hovered() || slider.dragged() || slider.has_focus() {
        st.highlight = Some(name.to_owned());
    }
    if st.model == BodyModel::Anny {
        delta::row(ui, theme, name, value, st.anny_solved.as_ref());
    }
}
