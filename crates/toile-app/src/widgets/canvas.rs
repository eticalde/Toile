use eframe::egui::{Align2, FontId, Painter, Rect, Response, Sense, Stroke, Ui, Vec2, pos2, vec2};

use crate::theme::Theme;

const GRID_STEP: f32 = 24.0;

/// The cutting mat: flat fill plus the ruled grid every 24 points.
pub fn mat_canvas(ui: &mut Ui, theme: &Theme, size: Vec2) -> (Response, Painter) {
    let (resp, painter) = ui.allocate_painter(size, Sense::click_and_drag());
    fill(&painter, theme, resp.rect);
    grid(&painter, theme, resp.rect, GRID_STEP, Vec2::ZERO);
    (resp, painter)
}

/// The bare mat under everything else.
pub fn fill(painter: &Painter, theme: &Theme, rect: Rect) {
    painter.rect_filled(rect, 0.0, theme.mat);
}

/// The ruled lines, `offset` sliding them so the mat can travel with a view.
pub fn grid(painter: &Painter, theme: &Theme, rect: Rect, step: f32, offset: Vec2) {
    let stroke = Stroke::new(1.0, theme.grid);
    let mut x = rect.left() + offset.x.rem_euclid(step);
    while x < rect.right() {
        painter.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], stroke);
        x += step;
    }
    let mut y = rect.top() + offset.y.rem_euclid(step);
    while y < rect.bottom() {
        painter.line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], stroke);
        y += step;
    }
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
