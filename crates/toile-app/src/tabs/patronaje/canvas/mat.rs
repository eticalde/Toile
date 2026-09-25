use eframe::egui::{self, Painter, Rect, Stroke, vec2};

use super::super::view::View;
use super::super::{ruler, snap};
use crate::theme::Theme;
use crate::widgets::grid;

/// The closest the mat draws its lines; under that they read as noise.
const GRID_MIN: f32 = 9.0;

/// The ruled lines, travelling with the view so a centimetre stays a
/// centimetre wherever the drawing has been dragged to.
///
/// The centimetre itself is drawn whenever there is room for it, so that the
/// grid on the mat is the grid the pointer catches; under that it falls back
/// to the decade the rulers are counting in.
pub(in crate::tabs::patronaje) fn ruled(p: &Painter, theme: &Theme, rect: Rect, view: View) {
    let fine = (snap::GRID_CM * view.scale()) as f32;
    let step = if fine >= GRID_MIN {
        fine
    } else {
        (ruler::step_cm(view.scale()) * view.scale() / 2.0) as f32
    };
    if step < GRID_MIN {
        return;
    }
    grid(
        p,
        theme,
        rect,
        step,
        view.to_screen([0.0, 0.0]) - rect.left_top(),
    );
}

/// The origin cross: the pattern's (0, 0), drawn over the grid so the centre a
/// draft is measured from is never in doubt. Each axis shows only while it
/// falls on the mat; the label only while their crossing does.
pub(super) fn origin(p: &Painter, theme: &Theme, rect: Rect, view: View) {
    let o = view.to_screen([0.0, 0.0]);
    let stroke = Stroke::new(1.0, theme.accent.gamma_multiply(0.55));
    if o.x >= rect.left() && o.x <= rect.right() {
        p.vline(o.x, rect.y_range(), stroke);
    }
    if o.y >= rect.top() && o.y <= rect.bottom() {
        p.hline(rect.x_range(), o.y, stroke);
    }
    if rect.contains(o) {
        p.text(
            o + vec2(4.0, 3.0),
            egui::Align2::LEFT_TOP,
            "0,0",
            egui::FontId::monospace(10.0),
            theme.muted,
        );
    }
}
