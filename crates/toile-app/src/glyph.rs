use eframe::egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, vec2};

/// Paints one icon into `slot`, in the ink the caller hands it.
///
/// An icon is data, not code: strokes separated by `;` inside a 16 × 16 box,
/// each a run of `x y` points, and `o x y r` for a circle.
pub fn paint(p: &Painter, slot: Rect, color: Color32, icon: &str) {
    let scale = slot.width() / 16.0;
    let at = |x: f32, y: f32| slot.left_top() + vec2(x, y) * scale;
    let stroke = Stroke::new(1.3, color);
    for part in icon.split(';') {
        let n: Vec<f32> = part
            .split_whitespace()
            .filter_map(|t| t.parse().ok())
            .collect();
        if part.trim_start().starts_with('o') {
            p.circle_stroke(at(n[0], n[1]), n[2] * scale, stroke);
        } else {
            let path: Vec<Pos2> = n.chunks(2).map(|c| at(c[0], c[1])).collect();
            p.add(Shape::line(path, stroke));
        }
    }
}
