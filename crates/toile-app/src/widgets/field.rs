use eframe::egui::{
    self, Align, Align2, FontId, Id, Painter, Rect, Sense, Stroke, StrokeKind, TextEdit, Ui, pos2,
    vec2,
};

use super::{CORNER, PAD};
use crate::theme::Theme;

const FIELD_H: f32 = 28.0;
const VALUE_W: f32 = 72.0;
const UNIT_W: f32 = 26.0;

/// Height of a row that carries a box over a line of its own.
const TALL_H: f32 = 50.0;

/// Label on the left, mono value box on the right, unit after it when given.
pub fn field_row(ui: &mut Ui, theme: &Theme, label: &str, value: &str, unit: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), FIELD_H), Sense::hover());
    let p = ui.painter();
    let unit_w = if unit.is_empty() { 0.0 } else { UNIT_W };
    let right = rect.right() - PAD - unit_w;
    let boxed = Rect::from_min_max(
        pos2(right - VALUE_W, rect.center().y - 11.0),
        pos2(right, rect.center().y + 11.0),
    );
    p.text(
        rect.left_center() + vec2(PAD, 0.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(12.0),
        theme.ink_soft,
    );
    value_box(p, theme, boxed, value);
    if !unit.is_empty() {
        p.text(
            pos2(right + 6.0, rect.center().y),
            Align2::LEFT_CENTER,
            unit,
            FontId::monospace(11.0),
            theme.muted,
        );
    }
}

/// One row of the inspector that can be written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Editable<'a> {
    /// The name on the left.
    pub label: &'a str,
    /// What the document holds, shown while nobody is writing in the row.
    pub source: &'a str,
    /// The line underneath: what it comes to, or why it does not.
    pub note: &'a str,
    /// Whether that line is a fault and not a measurement.
    pub fault: bool,
    /// The text the panel is keeping for this row, while this is the row it
    /// has the focus on.
    pub held: Option<&'a str>,
}

/// What a person did to an editable row this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edited {
    /// Nothing: nobody is writing in it.
    Idle,
    /// It has the focus and holds this text, which need not parse yet.
    Typing(String),
    /// Confirmed with this text, by Enter or by the focus moving on.
    Done(String),
}

/// One editable mono field over the line that says what it comes to.
///
/// The text lives with the caller and not in this widget, which is what lets
/// the row paint the fault in a half written formula while the document goes
/// on holding the last thing that parsed. Nothing is written until the row
/// answers `Done`.
pub fn formula_row(ui: &mut Ui, theme: &Theme, id: Id, row: &Editable<'_>) -> Edited {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), TALL_H), Sense::hover());
    let boxed = Rect::from_min_max(
        rect.left_top() + vec2(34.0, 4.0),
        pos2(rect.right() - PAD, rect.top() + 28.0),
    );
    let focused = ui.memory(|m| m.has_focus(id));
    let edge = if focused { theme.accent } else { theme.line };
    let p = ui.painter();
    p.rect(
        boxed,
        CORNER,
        theme.raised,
        Stroke::new(1.0, edge),
        StrokeKind::Inside,
    );
    p.text(
        pos2(rect.left() + PAD, boxed.center().y),
        Align2::LEFT_CENTER,
        row.label,
        FontId::proportional(12.0),
        theme.ink_soft,
    );
    let ink = if row.fault {
        theme.alert
    } else {
        theme.measure
    };
    p.text(
        pos2(boxed.left() + 2.0, boxed.bottom() + 11.0),
        Align2::LEFT_CENTER,
        row.note,
        FontId::monospace(11.0),
        ink,
    );
    let mut text = row.held.unwrap_or(row.source).to_owned();
    let resp = ui.put(
        boxed.shrink(1.0),
        TextEdit::singleline(&mut text)
            .id(id)
            .frame(egui::Frame::NONE)
            .margin(vec2(7.0, 0.0))
            .font(FontId::monospace(12.0))
            .text_color(theme.ink)
            .vertical_align(Align::Center)
            .desired_width(f32::INFINITY),
    );
    if resp.lost_focus() && row.held.is_some() {
        return Edited::Done(text);
    }
    if resp.has_focus() {
        return Edited::Typing(text);
    }
    Edited::Idle
}

fn value_box(p: &Painter, theme: &Theme, rect: Rect, value: &str) {
    p.rect(
        rect,
        CORNER,
        theme.raised,
        Stroke::new(1.0, theme.line),
        StrokeKind::Inside,
    );
    p.text(
        rect.right_center() - vec2(8.0, 0.0),
        Align2::RIGHT_CENTER,
        value,
        FontId::monospace(12.0),
        theme.ink,
    );
}
