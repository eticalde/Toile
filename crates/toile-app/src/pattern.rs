use eframe::egui;
use toile_engine::session::Session;

use crate::theme::Theme;
use crate::widgets;

/// Pixel radius within which a click grabs a control point.
const GRAB_RADIUS: f32 = 14.0;

/// Margin between the pattern's bounding box and the panel edge, in points.
const MARGIN: f64 = 40.0;

/// Draws the 2D pattern and applies drags straight to the session.
///
/// Every drag frame recompiles the rest state, so the 3D panel is already
/// showing the edit by the time the pointer moves again.
pub fn show(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    theme: &Theme,
    session: &mut Session,
    drag: &mut Option<(usize, egui::Vec2)>,
) {
    let (resp, painter) = widgets::mat_canvas(ui, theme, size);
    let rect = resp.rect;

    let contour: Vec<[f64; 2]> = session.contour().to_vec();
    let view = View::fit(&contour, rect);
    let pts: Vec<egui::Pos2> = contour.iter().map(|&p| view.to_screen(p)).collect();
    painter.add(egui::Shape::closed_line(
        pts.clone(),
        egui::Stroke::new(1.6, theme.outline),
    ));

    if resp.drag_started()
        && let Some(pos) = resp.interact_pointer_pos()
    {
        *drag = nearest(&pts, pos).map(|i| (i, pts[i] - pos));
    }
    if resp.drag_stopped() {
        *drag = None;
    }
    if let Some((i, offset)) = *drag
        && let Some(pos) = resp.interact_pointer_pos()
    {
        session.move_point(i, view.to_pattern(pos + offset));
    }

    for (i, q) in pts.iter().enumerate() {
        let (r, color) = if drag.is_some_and(|(j, _)| j == i) {
            (5.0, theme.alert)
        } else {
            (2.4, theme.accent)
        };
        painter.circle_filled(*q, r, color);
    }
    widgets::canvas_label(
        &painter,
        theme,
        rect,
        "PATRÓN 2D — arrastra un punto · las costuras en azul",
    );
}

fn nearest(pts: &[egui::Pos2], pos: egui::Pos2) -> Option<usize> {
    let mut best = (GRAB_RADIUS, None);
    for (i, q) in pts.iter().enumerate() {
        let d = q.distance(pos);
        if d < best.0 {
            best = (d, Some(i));
        }
    }
    best.1
}

/// Maps pattern metres, y up, onto panel points, y down.
struct View {
    centre: egui::Pos2,
    origin: (f64, f64),
    scale: f64,
}

impl View {
    fn fit(contour: &[[f64; 2]], rect: egui::Rect) -> Self {
        let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
        for p in contour {
            for k in 0..2 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        let span = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1.0e-6);
        Self {
            centre: rect.center(),
            origin: (f64::midpoint(lo[0], hi[0]), f64::midpoint(lo[1], hi[1])),
            scale: (f64::from(rect.width().min(rect.height())) - 2.0 * MARGIN) / span,
        }
    }

    fn to_screen(&self, p: [f64; 2]) -> egui::Pos2 {
        egui::pos2(
            self.centre.x + ((p[0] - self.origin.0) * self.scale) as f32,
            self.centre.y - ((p[1] - self.origin.1) * self.scale) as f32,
        )
    }

    fn to_pattern(&self, q: egui::Pos2) -> [f64; 2] {
        [
            self.origin.0 + f64::from(q.x - self.centre.x) / self.scale,
            self.origin.1 - f64::from(q.y - self.centre.y) / self.scale,
        ]
    }
}
