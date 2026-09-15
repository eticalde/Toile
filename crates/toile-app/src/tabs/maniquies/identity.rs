mod dialog;

use std::ops::RangeInclusive;

use eframe::egui::{
    self, Color32, FontId, Key, Painter, Rect, Response, RichText, Slider, SliderClamping, Stroke,
    TextEdit, vec2,
};
use toile_engine::body::ADULT_YEARS;
use toile_engine::session::Session;

pub use self::dialog::{NewManiqui, dialog};
use super::stand::{Control, NEW_NAME, Stand};
use crate::tabs::{Kept, UNNAMED};
use crate::theme::Theme;
use crate::widgets::{PAD, list_row_icon, section};

/// The width a slider's value box takes, matching the measures panel's own.
const VALUE_W: f32 = 78.0;

/// What the panel says of a body no product holds.
const LOOSE: &str = "Sin producto abierto: este maniquí no se guarda en ningún sitio y ningún \
                     patrón se resuelve contra él.";

/// The left panel: who the mannequin is, before how much it measures — the
/// name, the phenotype controls that shape it, and the button that opens the
/// new-mannequin dialog.
///
/// `estatura` is not one of these controls: it is one of the twenty
/// catalogue measurements, and the solver owns the `height` phenotype input
/// that answers to it (see `toile_engine::body::solve_anny`). A raw height
/// slider here would fight the measures panel's own row over the same number.
pub fn panel(ui: &mut egui::Ui, theme: &Theme, session: &mut Session, stand: &mut Stand) {
    section(ui, theme, "Maniquí");
    ui.add_space(4.0);
    name_field(ui, theme, session, stand);
    if Stand::kept(session) == Kept::Nowhere {
        note(ui, theme.alert, LOOSE);
    }
    if let Some(why) = stand.refused(session) {
        note(ui, theme.alert, &format!("rechazado: {why}"));
    }
    ui.add_space(6.0);
    shape_sliders(ui, theme, session, stand);

    ui.add_space(PAD);
    let stature = stand.stature_cm.map_or_else(
        || "Estatura resultante: —".to_owned(),
        |cm| format!("Estatura resultante: {cm:.1} cm"),
    );
    ui.horizontal(|ui| {
        ui.add_space(PAD);
        ui.label(RichText::new(stature).size(11.0).color(theme.ink_soft));
    });
    ui.add_space(PAD * 2.0);

    if list_row_icon(ui, theme, "Nuevo maniquí", false, plus_icon).clicked() {
        stand.new_dialog = Some(NewManiqui::named(Stand::free_name(session, NEW_NAME)));
    }
}

/// The body's name, written back to it when the field lets go of the focus.
///
/// The text stays with the stand while the field has the focus, so a name half
/// typed reaches nobody until it is confirmed; Escape walks away from it.
fn name_field(ui: &mut egui::Ui, theme: &Theme, session: &mut Session, stand: &mut Stand) {
    let mut text = stand
        .naming
        .clone()
        .unwrap_or_else(|| stand.body(session).name.clone());
    let field = ui
        .horizontal(|ui| {
            ui.add_space(PAD);
            ui.add(
                TextEdit::singleline(&mut text)
                    .id(egui::Id::new("maniqui-nombre"))
                    .font(FontId::proportional(15.0))
                    .text_color(theme.ink)
                    .hint_text(UNNAMED)
                    .desired_width(ui.available_width() - PAD),
            )
        })
        .inner;
    if field.lost_focus() {
        stand.naming = None;
        if !ui.input(|i| i.key_pressed(Key::Escape)) {
            stand.rename(session, &text);
        }
    } else if field.has_focus() {
        stand.naming = Some(text);
    } else {
        // egui takes the focus from a field that goes undrawn, and the frame
        // it lets go on is never seen from another tab: text left behind then
        // would be confirmed later onto whatever body is on the stand by then.
        stand.naming = None;
    }
}

/// The phenotype controls, read from the body's shape and written back as one.
fn shape_sliders(ui: &mut egui::Ui, theme: &Theme, session: &mut Session, stand: &mut Stand) {
    let mut shape = stand.shape(session);
    let unit = 0.0..=1.0;
    let touched = [
        slider(
            ui,
            theme,
            "Sexo (masculino → femenino)",
            &mut shape.sex,
            unit.clone(),
            "",
        ),
        slider(
            ui,
            theme,
            "Edad",
            &mut shape.age_years,
            ADULT_YEARS,
            " años",
        ),
        slider(
            ui,
            theme,
            "Complexión (delgada → robusta)",
            &mut shape.build,
            unit.clone(),
            "",
        ),
        slider(ui, theme, "Músculo", &mut shape.muscle, unit.clone(), ""),
        slider(
            ui,
            theme,
            "Proporciones (ideal → atípica)",
            &mut shape.proportions,
            unit,
            "",
        ),
    ];
    if touched.iter().any(Response::changed) {
        stand.grip(session, Control::Shape);
        stand.set_shape(session, shape);
        ui.ctx().request_repaint();
    }
    if touched.iter().any(super::pressed) {
        stand.keep(&Control::Shape);
    }
}

/// One phenotype slider: a label above, then a slider filling the panel's
/// width the same way the measures panel lays out a row.
///
/// A typed value lands when the box lets go of the focus, not on every key, so
/// it reaches the body as the one value it is and not as the digits on the way.
fn slider(
    ui: &mut egui::Ui,
    theme: &Theme,
    label: &str,
    value: &mut f64,
    range: RangeInclusive<f64>,
    suffix: &str,
) -> Response {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(PAD);
        ui.label(RichText::new(label).size(12.0).color(theme.ink_soft));
    });
    ui.horizontal(|ui| {
        ui.add_space(PAD);
        ui.spacing_mut().slider_width = (ui.available_width() - PAD - VALUE_W).max(60.0);
        ui.add(
            Slider::new(value, range)
                .suffix(suffix)
                .fixed_decimals(if suffix.is_empty() { 2 } else { 0 })
                .trailing_fill(true)
                .clamping(SliderClamping::Edits)
                .update_while_editing(false),
        )
    })
    .inner
}

/// A short paragraph under the name, wrapped to the panel.
fn note(ui: &mut egui::Ui, ink: Color32, text: &str) {
    let margin = egui::Margin::symmetric(PAD as i8, 4);
    egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        ui.label(RichText::new(text).size(11.0).color(ink));
    });
}

fn plus_icon(painter: &Painter, r: Rect, color: Color32) {
    let stroke = Stroke::new(1.4, color);
    let c = r.center();
    painter.line_segment([c - vec2(0.0, 5.0), c + vec2(0.0, 5.0)], stroke);
    painter.line_segment([c - vec2(5.0, 0.0), c + vec2(5.0, 0.0)], stroke);
}
