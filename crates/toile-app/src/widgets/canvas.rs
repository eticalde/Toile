use eframe::egui::{Align2, FontId, Painter, Rect, Response, Sense, Stroke, Ui, Vec2, pos2, vec2};

use crate::theme::Theme;

const GRID_STEP: f32 = 24.0;

/// The cutting mat: flat fill plus the ruled grid every 24 points.
pub fn mat_canvas(ui: &mut Ui, theme: &Theme, size: Vec2) -> (Response, Painter) {
    let (resp, painter) = ui.allocate_painter(size, Sense::click_and_drag());
    let rect = resp.rect;
    painter.rect_filled(rect, 0.0, theme.mat);
    let stroke = Stroke::new(1.0, theme.grid);
    let mut x = rect.left();
    while x < rect.right() {
        painter.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], stroke);
        x += GRID_STEP;
    }
    let mut y = rect.top();
    while y < rect.bottom() {
        painter.line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], stroke);
        y += GRID_STEP;
    }
    (resp, painter)
}

/// The mono caption pinned to a canvas' top-left corner.
pub fn canvas_label(painter: &Painter, theme: &Theme, rect: Rect, text: &str) {
    painter.text(
        rect.left_top() + vec2(10.0, 8.0),
        Align2::LEFT_TOP,
        text,
        FontId::monospace(11.0),
        theme.muted,
    );
}
