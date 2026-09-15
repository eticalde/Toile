use eframe::egui::{self, RichText, Slider, SliderClamping};
use toile_engine::body::{self, ADULT_YEARS};
use toile_engine::draft::{BodyShape, MeasureSet};
use toile_engine::session::Session;

use super::super::stand::{NEW_NAME, Stand};
use crate::tabs::Kept;
use crate::theme::Theme;

/// The fields the new-mannequin dialog edits while it is open.
pub struct NewManiqui {
    name: String,
    sex: f64,
    age_years: f64,
    stature_cm: f64,
    /// Whether the name field still has to be handed the keyboard. Set once,
    /// on the frame the dialog opens: asking for focus every frame would take
    /// it back from whatever the person clicked next, leaving the estatura
    /// box impossible to type into.
    focus_name: bool,
}

impl NewManiqui {
    /// The dialog as it opens: `name` filled in, and an adult of middling
    /// everything standing 170 cm.
    pub fn named(name: String) -> Self {
        let adult = BodyShape::default();
        Self {
            name,
            sex: adult.sex,
            age_years: adult.age_years,
            stature_cm: 170.0,
            focus_name: true,
        }
    }

    /// The body the fields describe, under `name`: the reference tape at the
    /// stature asked for.
    ///
    /// A shape is stored only once the sex or the age moved off the default,
    /// so a body made without touching either leaves the product in the format
    /// it was written in.
    fn body(&self, name: String) -> MeasureSet {
        let mut set = body::default_measures();
        set.name = name;
        set.values.insert("estatura".to_owned(), self.stature_cm);
        let shape = BodyShape {
            sex: self.sex,
            age_years: self.age_years,
            ..BodyShape::default()
        };
        if shape == BodyShape::default() {
            set
        } else {
            set.shaped(shape)
        }
    }
}

/// What "Crear" does with the body, said before it is pressed.
fn fate(kept: Kept) -> &'static str {
    match kept {
        Kept::InProduct => {
            "Se añade al producto y, desde ahora, su patrón se resuelve con él. El maniquí \
             anterior sigue en el producto."
        }
        Kept::Nowhere => {
            "No hay producto abierto: sustituye al maniquí de la mesa, que no se guarda en \
             ningún sitio."
        }
    }
}

/// The new-mannequin dialog, shown while `stand.new_dialog` is set.
///
/// Asks for a name, sex, age and stature, every one pre-filled so only the
/// stature ever has to be an intentional choice. A name another body of the
/// product carries cannot be created: bodies are chosen by name.
pub fn dialog(ctx: &egui::Context, theme: &Theme, session: &mut Session, stand: &mut Stand) {
    let Some(mut fields) = stand.new_dialog.take() else {
        return;
    };
    let kept = Stand::kept(session);
    let mut decided = None;
    egui::Modal::new(egui::Id::new("nuevo-maniqui")).show(ctx, |ui| {
        ui.set_width(340.0);
        ui.heading("Nuevo maniquí");
        ui.add_space(4.0);
        ui.label(RichText::new(fate(kept)).size(11.0).color(theme.ink_soft));
        ui.add_space(10.0);

        ui.label("Nombre");
        let name_field = ui.text_edit_singleline(&mut fields.name);
        if fields.focus_name {
            name_field.request_focus();
            fields.focus_name = false;
        }
        let typed = fields.name.trim();
        let taken = !typed.is_empty() && Stand::name_taken(session, typed);
        if taken {
            ui.label(
                RichText::new("Ya hay un maniquí con ese nombre en el producto.")
                    .size(11.0)
                    .color(theme.alert),
            );
        }
        ui.add_space(8.0);

        ui.label("Sexo (masculino → femenino)");
        ui.add(Slider::new(&mut fields.sex, 0.0..=1.0).fixed_decimals(2));
        ui.add_space(6.0);

        ui.label("Edad");
        ui.add(
            Slider::new(&mut fields.age_years, ADULT_YEARS)
                .suffix(" años")
                .fixed_decimals(0),
        );
        ui.add_space(6.0);

        ui.label("Estatura");
        let height_field = ui.add(
            Slider::new(&mut fields.stature_cm, 140.0..=210.0)
                .suffix(" cm")
                .fixed_decimals(1)
                .clamping(SliderClamping::Edits),
        );
        ui.add_space(12.0);

        let ready = !taken && fields.stature_cm.is_finite() && fields.stature_cm > 0.0;
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
            let typed = fields.name.trim();
            let name = if typed.is_empty() {
                Stand::free_name(session, NEW_NAME)
            } else {
                typed.to_owned()
            };
            stand.add(session, fields.body(name));
            ctx.request_repaint();
        }
        Some(false) => {}
        None => stand.new_dialog = Some(fields),
    }
}
