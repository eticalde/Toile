use eframe::egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};

pub(super) fn play_icon(p: &Painter, r: Rect, color: Color32) {
    let b = r.shrink(1.0);
    let tip = pos2(b.right(), b.center().y);
    let points = vec![b.left_top(), tip, b.left_bottom()];
    p.add(Shape::convex_polygon(points, color, Stroke::NONE));
}

pub(super) fn pause_icon(p: &Painter, r: Rect, color: Color32) {
    let b = r.shrink2(vec2(2.5, 1.0));
    let stroke = Stroke::new(1.6, color);
    p.line_segment([b.left_top(), b.left_bottom()], stroke);
    p.line_segment([b.right_top(), b.right_bottom()], stroke);
}

/// An almost closed circle, with the corner mark of an arrowhead at its start.
pub(super) fn reset_icon(p: &Painter, r: Rect, color: Color32) {
    let at = |x: f32, y: f32| r.left_top() + vec2(x, y) * r.width() / 16.0;
    let stroke = Stroke::new(1.3, color);
    let arc: Vec<Pos2> = (0..=16)
        .map(|i| {
            let a = (180.0 - 315.0 * i as f32 / 16.0).to_radians();
            at(8.0 + 5.0 * a.cos(), 8.0 + 5.0 * a.sin())
        })
        .collect();
    p.add(Shape::line(arc, stroke));
    p.add(Shape::line(
        vec![at(3.0, 3.0), at(3.0, 6.0), at(6.0, 6.0)],
        stroke,
    ));
}

pub(super) fn check_icon(p: &Painter, r: Rect, color: Color32) {
    let at = |x: f32, y: f32| r.left_top() + vec2(x, y) * r.width() / 12.0;
    let tick = vec![at(2.5, 6.5), at(5.0, 9.5), at(9.5, 3.0)];
    p.add(Shape::line(tick, Stroke::new(1.5, color)));
}

pub(super) fn warn_icon(p: &Painter, r: Rect, color: Color32) {
    let at = |x: f32, y: f32| r.left_top() + vec2(x, y) * r.width() / 12.0;
    let stroke = Stroke::new(1.2, color);
    let body = vec![at(6.0, 1.5), at(11.0, 10.5), at(1.0, 10.5)];
    p.add(Shape::closed_line(body, stroke));
    p.line_segment([at(6.0, 5.0), at(6.0, 8.5)], stroke);
}
