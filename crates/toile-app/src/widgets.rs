#![allow(
    dead_code,
    reason = "the panels that consume these helpers land tab by tab"
)]

mod canvas;

pub use canvas::{canvas_label, mat_canvas};
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Response, Sense, Stroke,
    StrokeKind, Ui, Vec2, pos2, vec2,
};

use crate::theme::Theme;

/// Horizontal breathing room inside a side panel, in points.
pub(crate) const PAD: f32 = 12.0;
const ROW_H: f32 = 26.0;
const FIELD_H: f32 = 28.0;
const VALUE_W: f32 = 72.0;
const UNIT_W: f32 = 26.0;
const ICON: f32 = 16.0;
/// The smaller glyph a button carries before its label.
const GLYPH: f32 = 12.0;
/// Where a label starts once a glyph precedes it.
const ICON_X: f32 = PAD + ICON + 8.0;
pub(crate) const CORNER: CornerRadius = CornerRadius::same(2);

// ── panel groups ──────────────────────────────────────────────────────────

/// Uppercase caption that opens a group inside a side panel.
pub fn section(ui: &mut Ui, theme: &Theme, title: &str) {
    section_with(ui, theme, title, "");
}

/// A section header carrying a right-aligned mono note (a count, a unit).
pub fn section_with(ui: &mut Ui, theme: &Theme, title: &str, extra: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 34.0), Sense::hover());
    let p = ui.painter();
    p.text(
        rect.left_bottom() + vec2(PAD, -6.0),
        Align2::LEFT_BOTTOM,
        title.to_uppercase(),
        FontId::monospace(10.0),
        theme.muted,
    );
    if !extra.is_empty() {
        p.text(
            rect.right_bottom() + vec2(-PAD, -6.0),
            Align2::RIGHT_BOTTOM,
            extra,
            FontId::monospace(11.0),
            theme.muted,
        );
    }
}

/// One entry of a library or a piece tree.
pub fn list_row(ui: &mut Ui, theme: &Theme, label: &str, selected: bool) -> Response {
    row(ui, theme, label, selected, PAD)
}

/// The same entry with a 16 pt glyph before the label, painted by the caller
/// into the slot it is handed.
pub fn list_row_icon(
    ui: &mut Ui,
    theme: &Theme,
    label: &str,
    selected: bool,
    icon: impl FnOnce(&Painter, Rect, Color32),
) -> Response {
    tree_row(ui, theme, label, selected, 0.0, icon)
}

/// The same entry pushed right by `indent`, one level down a tree.
pub fn tree_row(
    ui: &mut Ui,
    theme: &Theme,
    label: &str,
    selected: bool,
    indent: f32,
    icon: impl FnOnce(&Painter, Rect, Color32),
) -> Response {
    let resp = row(ui, theme, label, selected, ICON_X + indent);
    let slot = Rect::from_center_size(
        resp.rect.left_center() + vec2(PAD + indent + ICON / 2.0, 0.0),
        Vec2::splat(ICON),
    );
    icon(ui.painter(), slot, theme.muted);
    resp
}

fn row(ui: &mut Ui, theme: &Theme, label: &str, selected: bool, text_x: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::click());
    let tint = if selected {
        0.16
    } else if resp.hovered() {
        0.07
    } else {
        0.0
    };
    if tint > 0.0 {
        ui.painter()
            .rect_filled(rect, 0.0, theme.accent.gamma_multiply(tint));
    }
    ui.painter().text(
        rect.left_center() + vec2(text_x, 0.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        if selected { theme.ink } else { theme.ink_soft },
    );
    resp
}

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

/// Pinned to the foot of an inspector, under the rule that closes it.
pub fn footer_note(ui: &mut Ui, theme: &Theme, text: &str) {
    let margin = egui::Margin::symmetric(PAD as i8, 10);
    egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        let body = egui::RichText::new(text).monospace().size(11.0);
        ui.label(body.color(theme.muted));
    });
    rule(ui, theme);
}

/// The hairline that closes a group at the foot of a panel.
pub fn rule(ui: &mut Ui, theme: &Theme) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, theme.line);
}

// ── controls ──────────────────────────────────────────────────────────────

/// Caption plus a boxed value with a chevron: the sub-bar pickers.
pub fn select(ui: &mut Ui, theme: &Theme, caption: &str, value: &str, width: f32) -> Response {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(caption.to_uppercase())
                .monospace()
                .size(10.0)
                .extra_letter_spacing(1.2)
                .color(theme.muted),
        );
        let (rect, resp) = ui.allocate_exact_size(vec2(width, 24.0), Sense::click());
        let p = ui.painter();
        p.rect(
            rect,
            CORNER,
            theme.raised,
            Stroke::new(
                1.0,
                if resp.hovered() {
                    theme.muted
                } else {
                    theme.line
                },
            ),
            StrokeKind::Inside,
        );
        p.text(
            rect.left_center() + vec2(10.0, 0.0),
            Align2::LEFT_CENTER,
            value,
            FontId::proportional(12.0),
            theme.ink,
        );
        chevron(p, rect.right_center() - vec2(12.0, 0.0), theme.muted);
        resp
    })
    .inner
}

fn chevron(p: &Painter, centre: Pos2, color: Color32) {
    let s = Stroke::new(1.2, color);
    p.line_segment([centre + vec2(-4.0, -2.0), centre + vec2(0.0, 2.0)], s);
    p.line_segment([centre + vec2(4.0, -2.0), centre + vec2(0.0, 2.0)], s);
}

/// Filled call to action.
pub fn button_primary(ui: &mut Ui, theme: &Theme, label: &str) -> Response {
    button(ui, theme, label, true, 0.0).0
}

/// Outlined action, the default weight.
pub fn button_secondary(ui: &mut Ui, theme: &Theme, label: &str) -> Response {
    button(ui, theme, label, false, 0.0).0
}

/// Either weight with a 12 pt glyph before the label, painted by the caller
/// into the slot it is handed, in the ink the label already uses.
pub fn button_icon(
    ui: &mut Ui,
    theme: &Theme,
    label: &str,
    primary: bool,
    icon: impl FnOnce(&Painter, Rect, Color32),
) -> Response {
    let (resp, slot) = button(ui, theme, label, primary, GLYPH);
    let color = if primary { theme.on_accent } else { theme.ink };
    icon(ui.painter(), slot, color);
    resp
}

/// Returns the response and the glyph slot, empty when `glyph_w` is zero.
fn button(
    ui: &mut Ui,
    theme: &Theme,
    label: &str,
    primary: bool,
    glyph_w: f32,
) -> (Response, Rect) {
    let font = FontId::proportional(12.0);
    let text = ui.painter().layout_no_wrap(
        label.to_owned(),
        font,
        if primary { theme.on_accent } else { theme.ink },
    );
    let lead = if glyph_w > 0.0 { glyph_w + 6.0 } else { 0.0 };
    let size = vec2(text.size().x + lead + 28.0, 26.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let fill = if primary {
        theme.accent
    } else if resp.hovered() {
        theme.line
    } else {
        theme.panel
    };
    let stroke = Stroke::new(1.0, if primary { theme.accent } else { theme.line });
    ui.painter()
        .rect(rect, CORNER, fill, stroke, StrokeKind::Inside);
    let left = rect.center().x - f32::midpoint(text.size().x, lead);
    let top = rect.center().y - text.size().y / 2.0;
    let slot = Rect::from_center_size(
        pos2(left + glyph_w / 2.0, rect.center().y),
        Vec2::splat(glyph_w),
    );
    ui.painter().galley(pos2(left + lead, top), text, theme.ink);
    (resp, slot)
}
