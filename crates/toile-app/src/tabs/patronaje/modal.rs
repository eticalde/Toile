use eframe::egui::{self, Align2, FontId, Key, Rect, Stroke, StrokeKind, pos2, vec2};

use super::gesture::Ask;
use crate::theme::Theme;
use crate::widgets::{CORNER, PAD, button_primary, button_secondary};

const TITLE: &str = "Estás modificando una fórmula";
const ADAPT: &str = "Adaptar la fórmula";
const RESPECT: &str = "Respetar la fórmula";
const NOTE: &str = "El arrastre y esta decisión son una sola entrada de deshacer.";

/// The size of the card, in screen points; its height grows with its rows.
const CARD_W: f32 = 460.0;
const CARD_H: f32 = 138.0;
const ROW_H: f32 = 30.0;

/// What was answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// Keep the rewritten formula: the drag is absorbed by its adjustment.
    Adapt,
    /// Put the formula back the way it was written, and the node with it.
    Respect,
}

/// The question a drag over a formula asks before it is over.
///
/// There is no third way out on purpose: a gesture never breaks the link
/// between a coordinate and the measurements it is drawn from.
pub fn show(ui: &mut egui::Ui, theme: &Theme, rect: Rect, ask: &Ask) -> Option<Answer> {
    let height = CARD_H + ROW_H * ask.rows.len() as f32;
    let card = Rect::from_center_size(rect.center(), vec2(CARD_W, height));
    let p = ui.painter();
    p.rect_filled(rect, 0.0, theme.mat.gamma_multiply(0.75));
    p.rect(
        card,
        CORNER,
        theme.panel,
        Stroke::new(1.0, theme.accent),
        StrokeKind::Inside,
    );
    p.text(
        card.left_top() + vec2(PAD + 4.0, 22.0),
        Align2::LEFT_CENTER,
        TITLE,
        FontId::proportional(14.0),
        theme.ink,
    );
    for (index, row) in ask.rows.iter().enumerate() {
        let top = card.top() + 44.0 + ROW_H * index as f32;
        rewrite(p, theme, card, top, row);
    }
    p.text(
        pos2(card.left() + PAD + 4.0, card.bottom() - 46.0),
        Align2::LEFT_CENTER,
        NOTE,
        FontId::proportional(11.0),
        theme.muted,
    );
    buttons(ui, theme, card).or_else(|| keys(ui))
}

/// One coordinate, as it was written and as the drag left it.
fn rewrite(p: &egui::Painter, theme: &Theme, card: Rect, top: f32, row: &super::gesture::AskRow) {
    let font = FontId::monospace(12.0);
    let at = pos2(card.left() + PAD + 4.0, top + ROW_H / 2.0);
    p.text(
        at,
        Align2::LEFT_CENTER,
        format!("{}   {}", row.axis, row.before),
        font.clone(),
        theme.muted,
    );
    p.text(
        pos2(card.right() - PAD - 4.0, at.y),
        Align2::RIGHT_CENTER,
        format!("→   {}", row.after),
        font,
        theme.measure,
    );
}

/// The two ways out, in the corner they are always in.
fn buttons(ui: &mut egui::Ui, theme: &Theme, card: Rect) -> Option<Answer> {
    let bar = Rect::from_min_max(
        pos2(card.left() + PAD, card.bottom() - 38.0),
        pos2(card.right() - PAD, card.bottom() - 10.0),
    );
    let layout = egui::Layout::right_to_left(egui::Align::Center);
    ui.scope_builder(egui::UiBuilder::new().max_rect(bar).layout(layout), |ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if button_primary(ui, theme, ADAPT).clicked() {
            return Some(Answer::Adapt);
        }
        if button_secondary(ui, theme, RESPECT).clicked() {
            return Some(Answer::Respect);
        }
        None
    })
    .inner
}

/// The same two answers from the keyboard, where they always are.
fn keys(ui: &egui::Ui) -> Option<Answer> {
    ui.input(|i| {
        if i.key_pressed(Key::Enter) {
            return Some(Answer::Adapt);
        }
        i.key_pressed(Key::Escape).then_some(Answer::Respect)
    })
}
