use eframe::egui;
use toile_engine::session::Session;

/// Pixel radius within which a click grabs a control point.
const GRAB_RADIUS: f32 = 14.0;

/// Margin between the pattern's bounding box and the panel edge, in points.
const MARGIN: f64 = 40.0;

const BACKGROUND: egui::Color32 = egui::Color32::from_rgb(24, 26, 30);
const OUTLINE: egui::Color32 = egui::Color32::from_rgb(210, 210, 200);
const POINT: egui::Color32 = egui::Color32::from_rgb(95, 140, 235);
const POINT_ACTIVE: egui::Color32 = egui::Color32::from_rgb(235, 90, 80);
const LABEL: egui::Color32 = egui::Color32::from_rgb(140, 145, 150);

/// Draws the 2D pattern and applies drags straight to the session.
///
/// Every drag frame recompiles the rest state, so the 3D panel is already
/// showing the edit by the time the pointer moves again.
pub fn show(ui: &mut egui::Ui, size: egui::Vec2, session: &mut Session, drag: &mut Option<usize>) {
    let (resp, painter) = ui.allocate_painter(size, egui::Sense::click_and_drag());
    let rect = resp.rect;
    painter.rect_filled(rect, 0.0, BACKGROUND);

    let contour: Vec<[f64; 2]> = session.contour().to_vec();
    let view = View::fit(&contour, rect);
    let pts: Vec<egui::Pos2> = contour.iter().map(|&p| view.to_screen(p)).collect();
    painter.add(egui::Shape::closed_line(
        pts.clone(),
        egui::Stroke::new(1.6, OUTLINE),
    ));

    if resp.drag_started()
        && let Some(pos) = resp.interact_pointer_pos()
    {
        *drag = nearest(&pts, pos);
    }
    if resp.drag_stopped() {
        *drag = None;
    }
    if let Some(i) = *drag
        && let Some(pos) = resp.interact_pointer_pos()
    {
        session.move_point(i, view.to_pattern(pos));
    }

    for (i, q) in pts.iter().enumerate() {
        let (r, color) = if *drag == Some(i) {
            (5.0, POINT_ACTIVE)
        } else {
            (2.4, POINT)
        };
        painter.circle_filled(*q, r, color);
    }
    painter.text(
        rect.left_top() + egui::vec2(10.0, 8.0),
        egui::Align2::LEFT_TOP,
        "PATRÓN 2D — arrastra un punto",
        egui::FontId::monospace(11.0),
        LABEL,
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
