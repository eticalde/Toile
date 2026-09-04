use eframe::egui::{self, Align2, Color32, FontId, Painter, Rect, Response, Sense, Ui, Vec2, vec2};

use super::PAD;
use crate::theme::Theme;

const ROW_H: f32 = 26.0;
const ICON: f32 = 16.0;
/// Where a label starts once a glyph precedes it.
const ICON_X: f32 = PAD + ICON + 8.0;

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
