use eframe::egui::{
    self, Align2, Color32, FontId, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui,
    Vec2, pos2, vec2,
};

use super::CORNER;
use crate::theme::Theme;

/// The smaller glyph a button carries before its label.
const GLYPH: f32 = 12.0;

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
