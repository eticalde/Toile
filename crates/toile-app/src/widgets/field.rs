use eframe::egui::{
    Align2, Color32, FontId, Painter, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2,
};

use super::{CORNER, PAD};
use crate::theme::Theme;

const FIELD_H: f32 = 28.0;
const VALUE_W: f32 = 72.0;
const UNIT_W: f32 = 26.0;

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

/// One coordinate of a point: the formula, then what it resolves to.
pub fn formula_row(ui: &mut Ui, theme: &Theme, label: &str, formula: &str, resolved: &str) {
    coordinate(ui, theme, label, formula, resolved, theme.measure);
}

/// The same coordinate when its formula does not resolve: the line underneath
/// carries the fault instead of a measurement, in the ink that says so.
pub fn formula_row_fault(ui: &mut Ui, theme: &Theme, label: &str, formula: &str, fault: &str) {
    coordinate(ui, theme, label, formula, fault, theme.alert);
}

fn coordinate(ui: &mut Ui, theme: &Theme, label: &str, formula: &str, note: &str, ink: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 50.0), Sense::hover());
    let p = ui.painter();
    let boxed = Rect::from_min_max(
        rect.left_top() + vec2(34.0, 4.0),
        pos2(rect.right() - PAD, rect.top() + 28.0),
    );
    let line = Stroke::new(1.0, theme.line);
    p.rect(boxed, CORNER, theme.raised, line, StrokeKind::Inside);
    let at = pos2(rect.left() + PAD, boxed.center().y);
    let font = FontId::proportional(12.0);
    p.text(at, Align2::LEFT_CENTER, label, font, theme.ink_soft);
    let at = boxed.left_center() + vec2(8.0, 0.0);
    let font = FontId::monospace(12.0);
    p.text(at, Align2::LEFT_CENTER, formula, font, theme.ink);
    let at = pos2(boxed.left() + 2.0, boxed.bottom() + 11.0);
    let font = FontId::monospace(11.0);
    p.text(at, Align2::LEFT_CENTER, note, font, ink);
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
