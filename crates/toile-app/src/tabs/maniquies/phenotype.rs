use eframe::egui::{self, RichText, Slider, SliderClamping};
use toile_engine::body::BodyModel;

use super::State;
use crate::theme::Theme;
use crate::widgets::{PAD, section};

/// The width a phenotype slider's value box takes, matching `measures.rs`.
const VALUE_W: f32 = 78.0;

/// The six phenotype controls Anny's evaluator takes, shown only while the
/// Anny model is selected — the tailor's dummy has no such inputs. A change
/// marks the tab dirty, the same way a measurement edit does.
pub fn panel(ui: &mut egui::Ui, theme: &Theme, st: &mut State) {
    if st.model != BodyModel::Anny {
        return;
    }
    section(ui, theme, "Fenotipo");

    let mut changed = false;
    changed |= row(
        ui,
        theme,
        "Sexo (masculino → femenino)",
        &mut st.anny.sex,
        0.0,
        1.0,
        "",
    );
    changed |= row(
        ui,
        theme,
        "Edad",
        &mut st.anny.age_years,
        18.0,
        100.0,
        " años",
    );
    changed |= row(
        ui,
        theme,
        "Complexión (delgada → robusta)",
        &mut st.anny.weight,
        0.0,
        1.0,
        "",
    );
    changed |= row(ui, theme, "Músculo", &mut st.anny.muscle, 0.0, 1.0, "");
    changed |= row(
        ui,
        theme,
        "Altura (parámetro)",
        &mut st.anny.height,
        0.0,
        1.0,
        "",
    );
    changed |= row(
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

    ui.horizontal(|ui| {
        ui.add_space(PAD);
        ui.label(
            RichText::new(format!("Estatura resultante: {:.1} cm", st.anny_stature_cm))
                .size(11.0)
                .color(theme.ink_soft),
        );
    });
    ui.add_space(PAD);
}

/// One phenotype slider: a label above, then a slider filling the panel's
/// width the same way `measures::row` lays out a measurement, but editing a
/// plain `f64` rather than the measure set.
fn row(
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
    let slider = ui
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
    slider.changed() && value.is_finite()
}
