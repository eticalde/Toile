use eframe::egui::{self, Color32, Painter, Rect, RichText, Slider, SliderClamping, Stroke, vec2};

use super::{AnnyControls, State};
use crate::theme::Theme;
use crate::widgets::{PAD, list_row_icon, section};

/// The width a slider's value box takes, matching the measures panel's own.
const VALUE_W: f32 = 78.0;

/// The fields the new-mannequin dialog edits while it is open.
///
/// Every field starts at a sensible default so the dialog is submittable
/// the moment it opens: the only field that is ever truly required is
/// stature, not that the others start empty.
pub struct NewManiqui {
    nombre: String,
    sexo: f64,
    edad_years: f64,
    estatura_cm: f64,
    /// Whether the name field still has to be handed the keyboard. Set once,
    /// on the frame the dialog opens: asking for focus every frame would take
    /// it back from whatever the person clicked next, leaving the estatura
    /// box impossible to type into.
    focus_name: bool,
}

impl Default for NewManiqui {
    fn default() -> Self {
        Self {
            nombre: "Maniquí".to_owned(),
            sexo: 0.5,
            edad_years: 25.0,
            estatura_cm: 170.0,
            focus_name: true,
        }
    }
}

/// The left panel: who the mannequin is, before how much it measures — the
/// name, the phenotype controls that shape it, and the button that opens
/// the new-mannequin dialog.
///
/// `estatura` is not one of these controls: it is one of the twenty
/// catalogue measurements, and the solver owns the `height` phenotype
/// input that answers to it (see `toile_engine::body::solve_anny`). A raw
/// height slider here would fight the measures panel's own row over the
/// same number.
pub fn panel(ui: &mut egui::Ui, theme: &Theme, st: &mut State) {
    section(ui, theme, "Maniquí");
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space(PAD);
        ui.label(RichText::new(&st.name).size(15.0).color(theme.ink));
    });
    ui.add_space(10.0);

    let mut changed = false;
    changed |= slider(
        ui,
        theme,
        "Sexo (masculino → femenino)",
        &mut st.anny.sex,
        0.0,
        1.0,
        "",
    );
    changed |= slider(
        ui,
        theme,
        "Edad",
        &mut st.anny.age_years,
        18.0,
        100.0,
        " años",
    );
    changed |= slider(
        ui,
        theme,
        "Complexión (delgada → robusta)",
        &mut st.anny.weight,
        0.0,
        1.0,
        "",
    );
    changed |= slider(ui, theme, "Músculo", &mut st.anny.muscle, 0.0, 1.0, "");
    changed |= slider(
        ui,
        theme,
        "Proporciones (ideal → atípica)",
        &mut st.anny.proportions,
        0.0,
        1.0,
        "",
    );
    if changed {
        st.dirty = true;
        ui.ctx().request_repaint();
    }

    ui.add_space(PAD);
    ui.horizontal(|ui| {
        ui.add_space(PAD);
        ui.label(
            RichText::new(format!("Estatura resultante: {:.1} cm", st.anny_stature_cm))
                .size(11.0)
                .color(theme.ink_soft),
        );
    });
    ui.add_space(PAD * 2.0);

    if list_row_icon(ui, theme, "Nuevo maniquí", false, plus_icon).clicked() {
        st.new_dialog = Some(NewManiqui::default());
    }
}

/// One phenotype slider: a label above, then a slider filling the panel's
/// width the same way the measures panel lays out a row. Returns whether
/// the edit just committed — the slider released, a click landed, or the
/// value box lost focus — which is when the caller should re-solve, not
/// on every frame a drag merely continues (see `super::build`'s own doc).
fn slider(
    ui: &mut egui::Ui,
    theme: &Theme,
    label: &str,
    value: &mut f64,
    lo: f64,
    hi: f64,
    suffix: &str,
) -> bool {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(PAD);
        ui.label(RichText::new(label).size(12.0).color(theme.ink_soft));
    });
    let resp = ui
        .horizontal(|ui| {
            ui.add_space(PAD);
            ui.spacing_mut().slider_width = (ui.available_width() - PAD - VALUE_W).max(60.0);
            ui.add(
                Slider::new(value, lo..=hi)
                    .suffix(suffix)
                    .fixed_decimals(if suffix.is_empty() { 2 } else { 0 })
                    .trailing_fill(true)
                    .clamping(SliderClamping::Edits),
            )
        })
        .inner;
    (resp.drag_stopped() || resp.lost_focus() || resp.clicked()) && value.is_finite()
}

/// The new-mannequin dialog, shown while `st.new_dialog` is set.
///
/// Asks for a nombre, sexo, edad and estatura, every one pre-filled so
/// only estatura ever has to be an intentional choice, and even that keeps
/// whatever default or edit was last in the box: nothing here can be left
/// blank. There is no persona library yet, so "Crear" replaces the
/// mannequin on the table; the dialog says so plainly rather than losing
/// the current one silently.
pub fn dialog(ctx: &egui::Context, theme: &Theme, st: &mut State) {
    let Some(mut draft) = st.new_dialog.take() else {
        return;
    };
    let mut decided = None;
    egui::Modal::new(egui::Id::new("nuevo-maniqui")).show(ctx, |ui| {
        ui.set_width(340.0);
        ui.heading("Nuevo maniquí");
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "Sustituye al maniquí de la mesa: todavía no hay una biblioteca donde \
                 guardar varios.",
            )
            .size(11.0)
            .color(theme.ink_soft),
        );
        ui.add_space(10.0);

        ui.label("Nombre");
        let name_field = ui.text_edit_singleline(&mut draft.nombre);
        if draft.focus_name {
            name_field.request_focus();
            draft.focus_name = false;
        }
        ui.add_space(8.0);

        ui.label("Sexo (masculino → femenino)");
        ui.add(Slider::new(&mut draft.sexo, 0.0..=1.0).fixed_decimals(2));
        ui.add_space(6.0);

        ui.label("Edad");
        ui.add(
            Slider::new(&mut draft.edad_years, 18.0..=100.0)
                .suffix(" años")
                .fixed_decimals(0),
        );
        ui.add_space(6.0);

        ui.label("Estatura");
        let height_field = ui.add(
            Slider::new(&mut draft.estatura_cm, 140.0..=210.0)
                .suffix(" cm")
                .fixed_decimals(1)
                .clamping(SliderClamping::Edits),
        );
        ui.add_space(12.0);

        let ready = draft.estatura_cm.is_finite() && draft.estatura_cm > 0.0;
        if height_field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && ready {
            decided = Some(true);
        }
        ui.horizontal(|ui| {
            if ui.button("Cancelar").clicked() {
                decided = Some(false);
            }
            if ui.add_enabled(ready, egui::Button::new("Crear")).clicked() {
                decided = Some(true);
            }
        });
    });
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        decided = Some(false);
    }
    match decided {
        Some(true) => {
            st.name = if draft.nombre.trim().is_empty() {
                "Maniquí".to_owned()
            } else {
                draft.nombre.trim().to_owned()
            };
            st.measures = toile_engine::body::default_measures();
            st.measures
                .values
                .insert("estatura".to_owned(), draft.estatura_cm);
            st.anny = AnnyControls {
                sex: draft.sexo,
                age_years: draft.edad_years,
                weight: 0.5,
                muscle: 0.5,
                proportions: 0.5,
            };
            st.dirty = true;
        }
        Some(false) => {}
        None => st.new_dialog = Some(draft),
    }
}

fn plus_icon(painter: &Painter, r: Rect, color: Color32) {
    let stroke = Stroke::new(1.4, color);
    let c = r.center();
    painter.line_segment([c - vec2(0.0, 5.0), c + vec2(0.0, 5.0)], stroke);
    painter.line_segment([c - vec2(5.0, 0.0), c + vec2(5.0, 0.0)], stroke);
}
