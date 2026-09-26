mod delta;

use eframe::egui::{self, RichText, Slider, SliderClamping};
use toile_engine::session::Session;

use super::stand::{Control, Stand};
use crate::tabs::{UNNAMED, catalogue};
use crate::theme::Theme;
use crate::widgets::{PAD, footer_note, section, section_with};

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

/// The editable inspector: every catalogue measurement the body carries as a
/// slider with its value box, and every one it does not as what the body
/// measures there. A value goes to the body through the stand; a row under the
/// pointer or in hand lays its tape on the body.
///
/// Twenty rows outgrow any window height, so the groups scroll under a pinned
/// title; the sections keep their order, so the scroll position is the only
/// thing that moves.
pub fn panel(ui: &mut egui::Ui, theme: &Theme, session: &mut Session, stand: &mut Stand) {
    let set = stand.body(session);
    let named = if set.name.is_empty() {
        UNNAMED
    } else {
        &set.name
    };
    section_with(ui, theme, &format!("Medidas · {named}"), "cm");
    footer_note(
        ui,
        theme,
        "El fenotipo da forma a este cuerpo; cada fila dice cuánto mide de verdad y a qué \
         distancia queda de tu cinta. Escribir un valor mueve las palancas que pueden \
         alcanzarlo.",
    );
    let lit = egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut lit = None;
            for (heading, names) in catalogue::KINDS {
                section(ui, theme, heading);
                for &name in names {
                    lit = row(ui, theme, session, stand, (name, catalogue::label(name))).or(lit);
                }
            }
            ui.add_space(PAD);
            lit
        })
        .inner;
    // The tape belongs to the hand. No row answering means the pointer has
    // left the panel, and the body goes bare rather than wearing the last tape
    // laid on it: a mark nobody is pointing at reads as a mark that is stuck.
    stand.highlight = lit;
}

/// One measurement: its label over a slider that fills the panel. The label
/// takes the accent while the row's tape is the one on the body, drawn in the
/// same colour.
///
/// A measurement the body does not carry gets no slider. A product's body
/// holds the measurements it was given, and a slider that added one would
/// write an edit its own undo could not take back.
fn row(
    ui: &mut egui::Ui,
    theme: &Theme,
    session: &mut Session,
    stand: &mut Stand,
    entry: (&str, &str),
) -> Option<String> {
    let (name, label) = entry;
    let carried = stand.body(session).get(name);
    let mut value = carried.unwrap_or_default();
    let (lo, hi) = span(name);
    let lit = stand.highlight.as_deref() == Some(name);
    let ink = if lit { theme.accent } else { theme.ink_soft };

    let scoped = ui.scope(|ui| {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(PAD);
            ui.label(RichText::new(label).size(12.0).color(ink));
        });
        carried.is_some().then(|| {
            ui.horizontal(|ui| {
                ui.add_space(PAD);
                ui.spacing_mut().slider_width = (ui.available_width() - PAD - VALUE_W).max(60.0);
                ui.add(
                    Slider::new(&mut value, lo..=hi)
                        .suffix(" cm")
                        .fixed_decimals(1)
                        .trailing_fill(true)
                        .clamping(SliderClamping::Edits)
                        .update_while_editing(false),
                )
            })
            .inner
        })
    });

    // The row under the pointer, the slider in hand, or the box with the
    // focus: that is the one whose tape lies on the body, and it answers so
    // the panel can put the light out when none of them does.
    let mut handled = scoped.response.hovered();
    if let Some(slider) = scoped.inner {
        // Every frame the value moves reaches the document, inside the one
        // gesture the grip opened; the body waits for the gesture to close.
        if slider.changed() && value.is_finite() {
            stand.grip(session, Control::Measure(name.to_owned()));
            stand.set_measure(session, name, value);
            ui.ctx().request_repaint();
        }
        if super::pressed(&slider) {
            stand.keep(&Control::Measure(name.to_owned()));
        }
        handled |= slider.hovered() || slider.dragged() || slider.has_focus();
    }
    delta::row(
        ui,
        theme,
        name,
        carried.map(|_| value),
        stand.solved.as_ref(),
    );
    handled.then(|| name.to_owned())
}
