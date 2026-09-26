use eframe::egui::{self, Id};

use super::super::state::State;
use crate::config::Paper;
use crate::file::Action;
use crate::theme::Theme;
use crate::widgets::{PAD, button_named, cycle_named, section};

/// How wide the box that names the paper is, in points: room for the longer of
/// the two names and for the mark that steps them.
const PAPER_W: f32 = 96.0;

/// The identity each control of the section is drawn under, so a press aimed
/// from outside the panel finds one by what it does and never by where the rows
/// above it happened to leave it.
pub(super) fn paper_id() -> Id {
    Id::new("papel-de-las-hojas")
}

pub(super) fn pdf_id() -> Id {
    Id::new("exportar-pdf")
}

pub(super) fn svg_id() -> Id {
    Id::new("exportar-svg")
}

/// The ways a pattern leaves the app, both of them at true scale and both of
/// them over the whole product: the sheets of paper every piece is tiled onto,
/// and the drawing another program reads.
///
/// The paper first, because it is what a garment is cut from.
///
/// The size of paper is read and stepped here and nowhere else. It is the one
/// thing a print needs that no document holds — a ream belongs to the studio
/// and not to the garment — and a person who cannot say it prints a page taller
/// than the paper they own, on sheets that each ask to be printed at 100 %.
/// Stepped above the button rather than written into its label, so what the
/// button promises is one thing and not one thing per size of paper.
pub(super) fn show(ui: &mut egui::Ui, theme: &Theme, state: &mut State, paper: Paper) {
    section(ui, theme, "Exportar");
    ui.horizontal(|ui| {
        ui.add_space(PAD);
        if cycle_named(ui, theme, paper_id(), "papel", paper.name(), PAPER_W).clicked() {
            state.asked = Some(Action::Paper);
            // The app steps the choice once this panel has drawn, and a step
            // that opens no dialogue sends nothing anywhere: without this the
            // box would sit on the old size until something else asked for a
            // frame.
            ui.ctx().request_repaint();
        }
    });
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.add_space(PAD);
        if button_named(ui, theme, pdf_id(), "PDF · 1:1").clicked() {
            state.asked = Some(Action::Pdf);
        }
        if button_named(ui, theme, svg_id(), "SVG").clicked() {
            state.asked = Some(Action::Svg);
        }
    });
}
