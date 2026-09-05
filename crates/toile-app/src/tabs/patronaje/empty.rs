use eframe::egui::{self, Align, Align2, FontId, Layout, Rect, UiBuilder, vec2};

use crate::file::Action;
use crate::theme::Theme;
use crate::widgets::{button_primary, button_secondary};

const TITLE: &str = "Mesa vacía";
const NOTE: &str = "Toile no decide qué estás patronando.";

/// The three ways onto the table, in the order a person tries them.
const WAYS: [(&str, Action); 3] = [
    ("Nuevo producto", Action::New),
    ("Abrir…", Action::Open),
    ("Ejemplos · pantalón base", Action::Example),
];

/// Room between two of them, in points.
const GAP: f32 = 8.0;

/// What a button of `label` takes across, the way the widget lays it out.
const PADDING: f32 = 28.0;

/// The empty table: the three ways to put a pattern on it.
///
/// The drafting tab opens with nothing on the mat on purpose, so this is the
/// whole of the way out of it, and the only place the block Toile ships is
/// offered.
pub fn show(ui: &mut egui::Ui, theme: &Theme, rect: Rect) -> Option<Action> {
    let p = ui.painter();
    p.text(
        rect.center() - vec2(0.0, 46.0),
        Align2::CENTER_CENTER,
        TITLE,
        FontId::proportional(17.0),
        theme.ink,
    );
    p.text(
        rect.center() - vec2(0.0, 22.0),
        Align2::CENTER_CENTER,
        NOTE,
        FontId::proportional(12.0),
        theme.muted,
    );
    ways(ui, theme, rect)
}

/// The row of ways, measured before it is drawn so that it is centred on the
/// mat and not on whatever egui had left over.
fn ways(ui: &mut egui::Ui, theme: &Theme, rect: Rect) -> Option<Action> {
    let width = strip_width(ui);
    let strip = Rect::from_center_size(rect.center() + vec2(0.0, 12.0), vec2(width, 30.0));
    let layout = Layout::left_to_right(Align::Center);
    ui.scope_builder(UiBuilder::new().max_rect(strip).layout(layout), |ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        let mut asked = None;
        for (index, (label, action)) in WAYS.into_iter().enumerate() {
            let hit = if index == 0 {
                button_primary(ui, theme, label)
            } else {
                button_secondary(ui, theme, label)
            };
            if hit.clicked() {
                asked = Some(action);
            }
        }
        asked
    })
    .inner
}

/// How wide the three ways lie side by side.
fn strip_width(ui: &egui::Ui) -> f32 {
    let font = FontId::proportional(12.0);
    let labels: f32 = WAYS
        .iter()
        .map(|(label, _)| {
            ui.painter()
                .layout_no_wrap((*label).to_owned(), font.clone(), egui::Color32::WHITE)
                .size()
                .x
                + PADDING
        })
        .sum();
    labels + GAP * (WAYS.len() as f32 - 1.0)
}
