use eframe::egui::{
    self, Align2, Color32, FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind,
    Ui, Vec2, pos2, vec2,
};

use super::CORNER;
use crate::theme::Theme;

/// The smaller glyph a button carries before its label.
const GLYPH: f32 = 12.0;

/// Caption plus a boxed value: something the app knows, with nothing to press.
pub fn readout(ui: &mut Ui, theme: &Theme, caption: &str, value: &str, width: f32) {
    boxed(ui, theme, caption, value, width, None);
}

/// The same box over a value a press takes to the next one in order.
///
/// Marked with a cycle and never a chevron. A chevron promises a list to
/// choose from, this app opens no menu anywhere, and a press here does not
/// choose: it steps. The mark is what the press really does.
pub fn cycle(ui: &mut Ui, theme: &Theme, caption: &str, value: &str, width: f32) -> Response {
    boxed(ui, theme, caption, value, width, Some(cycle_mark))
}

/// Caption, boxed value, and the mark that says how the box answers a press.
///
/// The mark is what carries the click: a box drawn without one senses nothing,
/// so a control that has nothing to do cannot be pressed and left wondering.
fn boxed(
    ui: &mut Ui,
    theme: &Theme,
    caption: &str,
    value: &str,
    width: f32,
    mark: Option<fn(&Painter, Pos2, Color32)>,
) -> Response {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(caption.to_uppercase())
                .monospace()
                .size(10.0)
                .extra_letter_spacing(1.2)
                .color(theme.muted),
        );
        let live = mark.is_some();
        let sense = if live { Sense::click() } else { Sense::hover() };
        let (rect, resp) = ui.allocate_exact_size(vec2(width, 24.0), sense);
        let p = ui.painter();
        // The border lifts under the pointer only where a press does
        // something: a box that answers nothing may not brighten as if it did.
        let edge = if live && resp.hovered() {
            theme.muted
        } else {
            theme.line
        };
        p.rect(
            rect,
            CORNER,
            theme.raised,
            Stroke::new(1.0, edge),
            StrokeKind::Inside,
        );
        p.text(
            rect.left_center() + vec2(10.0, 0.0),
            Align2::LEFT_CENTER,
            value,
            FontId::proportional(12.0),
            theme.ink,
        );
        if let Some(mark) = mark {
            mark(p, rect.right_center() - vec2(12.0, 0.0), theme.muted);
        }
        resp
    })
    .inner
}

/// A ring left open, with an arrowhead at the head of the turn.
fn cycle_mark(p: &Painter, centre: Pos2, color: Color32) {
    let stroke = Stroke::new(1.2, color);
    let (from, to) = (-45.0_f32, 255.0_f32);
    let at = |deg: f32| centre + Vec2::angled(deg.to_radians()) * 4.2;
    let arc: Vec<Pos2> = (0..=14)
        .map(|i| at(from + (to - from) * i as f32 / 14.0))
        .collect();
    p.add(Shape::line(arc, stroke));
    let tip = at(to);
    let wing = |turn: f32| tip + Vec2::angled((to + 90.0 + turn).to_radians()) * 3.0;
    p.add(Shape::line(vec![wing(150.0), tip, wing(-150.0)], stroke));
}

/// Filled call to action.
pub fn button_primary(ui: &mut Ui, theme: &Theme, label: &str) -> Response {
    button(ui, theme, label, true, 0.0).0
}

/// Outlined action, the default weight.
pub fn button_secondary(ui: &mut Ui, theme: &Theme, label: &str) -> Response {
    button(ui, theme, label, false, 0.0).0
}

/// An action whose phase has not arrived: the same room, no border, muted ink,
/// and no click to answer with, because a button that lies is worse than a
/// gap.
pub fn button_ghost(ui: &mut Ui, theme: &Theme, label: &str) {
    ghost(ui, theme, label, 0.0);
}

/// The same dead weight, carrying the glyph that will stand for the action.
pub fn button_ghost_icon(
    ui: &mut Ui,
    theme: &Theme,
    label: &str,
    icon: impl FnOnce(&Painter, Rect, Color32),
) {
    let slot = ghost(ui, theme, label, GLYPH);
    icon(ui.painter(), slot, theme.muted);
}

/// Lays a dead button out the way a live one is laid out, and hands back its
/// glyph slot: what separates the two is the border, the ink and the sense,
/// never the room they take, so one does not shift when the other arrives.
fn ghost(ui: &mut Ui, theme: &Theme, label: &str, glyph_w: f32) -> Rect {
    let font = FontId::proportional(12.0);
    let text = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, theme.muted);
    let lead = if glyph_w > 0.0 { glyph_w + 6.0 } else { 0.0 };
    let size = vec2(text.size().x + lead + 28.0, 26.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let left = rect.center().x - f32::midpoint(text.size().x, lead);
    let top = rect.center().y - text.size().y / 2.0;
    ui.painter()
        .galley(pos2(left + lead, top), text, theme.muted);
    Rect::from_center_size(
        pos2(left + glyph_w / 2.0, rect.center().y),
        Vec2::splat(glyph_w),
    )
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
