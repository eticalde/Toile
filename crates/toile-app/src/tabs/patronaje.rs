use eframe::egui::{
    self, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Vec2,
    pos2, vec2,
};
use toile_engine::session::Session;

use crate::tabs::{Workspace, left_panel, right_panel};
use crate::theme::Theme;
use crate::widgets::{
    CORNER, PAD, button_secondary, canvas_label, field_row, footer_note, mat_canvas, section,
    section_with, select, tree_row,
};

/// An icon is data, not code: strokes separated by `;` inside a 16 × 16 box,
/// each a run of `x y` points, and `o x y r` for a circle.
const TOOLS: [(&str, &str); 9] = [
    ("Seleccionar", "3 2 13 8 8.8 9 7 13 3 2"),
    ("Punto", "o 8 8 2.5"),
    ("Recta", "3.5 12.5 12.5 3.5; o 3.5 12 1.4; o 12 3.5 1.4"),
    ("Curva", "2 13 5 12 8 9 11 4 14 3; 2 13 5 9; o 5 9 1.3"),
    ("Pinza", "3 3 8 13 13 3"),
    ("Piquete", "2 9 14 9; 8 9 8 5"),
    ("Espejo", "8 2 8 14; 5 5 2 8 5 11; 11 5 14 8 11 11"),
    ("Medir", "2 6 14 6 14 10 2 10 2 6; 5 6 5 8; 11 6 11 8"),
    ("Coser", "2 11 5 6 8 11 11 6 14 11"),
];
const PIECE_ICON: &str = "4 2 10 2 13 6 13 14 4 14 4 2";
const PIECES: [&str; 4] = ["Delantero", "Trasero", "Pretina", "Bragueta"];
const CHEVRON: &str = "6 4 10 8 6 12";
const PLUS: &str = "8 3 8 13; 3 8 13 8";

const MEASURES: [(&str, &str); 4] = [
    ("cintura", "84.0"),
    ("cadera", "98.0"),
    ("tiro", "27.0"),
    ("altura_cadera", "20.0"),
];
const NOTE: &str =
    "Las fórmulas se evalúan contra el maniquí elegido: mismo patrón, cualquier talla.";

/// Control points drawn on the contour, matching the count the status bar
/// reports, and which of them carries the drag handle.
const POINTS: usize = 9;
const GRABBED: usize = 2;

pub fn show(ui: &mut egui::Ui, w: &mut Workspace<'_>) {
    let theme = w.theme;
    left_panel(ui, theme, |ui| {
        product(ui, theme);
        tools(ui, theme);
    });
    right_panel(ui, theme, |ui| inspector(ui, theme));
    centre(ui, theme, w.session);
}

// ── panels ────────────────────────────────────────────────────────────────

fn product(ui: &mut egui::Ui, theme: &Theme) {
    section(ui, theme, "Producto");
    tree_row(ui, theme, "Pantalón base", false, 0.0, |p, r, c| {
        glyph(p, r, c, CHEVRON);
    });
    for (i, name) in PIECES.iter().enumerate() {
        tree_row(ui, theme, name, i == 0, 14.0, |p, r, c| {
            glyph(p, r, c, PIECE_ICON);
        });
    }
    ghost_row(ui, theme, "Pieza");
}

/// The "add a piece" affordance: a hint rather than an item, so it stays muted.
fn ghost_row(ui: &mut egui::Ui, theme: &Theme, label: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::hover());
    let p = ui.painter();
    let slot = Rect::from_center_size(rect.left_center() + vec2(34.0, 0.0), Vec2::splat(16.0));
    glyph(p, slot, theme.muted, PLUS);
    let at = rect.left_center() + vec2(50.0, 0.0);
    let font = FontId::proportional(13.0);
    p.text(at, Align2::LEFT_CENTER, label, font, theme.muted);
}

fn tools(ui: &mut egui::Ui, theme: &Theme) {
    section(ui, theme, "Herramientas");
    let width = (ui.available_width() - 2.0 * PAD - 8.0) / 3.0;
    for (i, row) in TOOLS.chunks(3).enumerate() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.add_space(PAD);
            for (j, (name, icon)) in row.iter().enumerate() {
                tile(ui, theme, name, icon, i + j == 0, width);
            }
        });
        ui.add_space(4.0);
    }
}

fn tile(ui: &mut egui::Ui, theme: &Theme, name: &str, icon: &str, active: bool, width: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 48.0), Sense::hover());
    let p = ui.painter();
    let ink = if active { theme.ink } else { theme.ink_soft };
    let edge = if active { theme.accent } else { theme.line };
    if active {
        p.rect_filled(rect, CORNER, theme.accent.gamma_multiply(0.16));
    }
    p.rect_stroke(rect, CORNER, Stroke::new(1.0, edge), StrokeKind::Inside);
    let slot = Rect::from_center_size(rect.center_top() + vec2(0.0, 17.0), Vec2::splat(16.0));
    glyph(p, slot, ink, icon);
    let at = rect.center_bottom() - vec2(0.0, 12.0);
    let font = FontId::proportional(10.0);
    p.text(at, Align2::CENTER_CENTER, name, font, ink);
}

fn inspector(ui: &mut egui::Ui, theme: &Theme) {
    section(ui, theme, "Punto P3");
    formula_row(ui, theme, "X", "cintura / 4 + 1", "= 22.0 cm");
    formula_row(ui, theme, "Y", "altura_cadera - 1", "= 19.0 cm");
    section_with(ui, theme, "Medidas del producto", "4");
    ui.horizontal(|ui| {
        ui.add_space(PAD);
        select(ui, theme, "resolver con", "Etienne", 170.0);
    });
    ui.add_space(6.0);
    for (label, value) in MEASURES {
        field_row(ui, theme, label, value, "cm");
    }
    section(ui, theme, "Exportar");
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.add_space(PAD);
        button_secondary(ui, theme, "PDF A4 · 1:1");
        button_secondary(ui, theme, "SVG");
    });
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        footer_note(ui, theme, NOTE);
    });
}

/// One coordinate of a point: the formula, then what it resolves to.
fn formula_row(ui: &mut egui::Ui, theme: &Theme, label: &str, formula: &str, resolved: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 50.0), Sense::hover());
    let p = ui.painter();
    let boxed = Rect::from_min_max(
        rect.left_top() + vec2(34.0, 4.0),
        pos2(rect.right() - PAD, rect.top() + 28.0),
    );
    let line = Stroke::new(1.0, theme.line);
    p.rect(boxed, CORNER, theme.raised, line, StrokeKind::Inside);
    let at = pos2(rect.left() + PAD, boxed.center().y);
    let font = FontId::proportional(12.0);
    p.text(at, Align2::LEFT_CENTER, label, font, theme.ink_soft);
    let at = boxed.left_center() + vec2(8.0, 0.0);
    let font = FontId::monospace(12.0);
    p.text(at, Align2::LEFT_CENTER, formula, font, theme.ink);
    let at = pos2(boxed.left() + 2.0, boxed.bottom() + 11.0);
    let font = FontId::monospace(11.0);
    p.text(at, Align2::LEFT_CENTER, resolved, font, theme.measure);
}

// ── canvas ────────────────────────────────────────────────────────────────

fn centre(ui: &mut egui::Ui, theme: &Theme, session: &Session) {
    egui::CentralPanel::no_frame().show(ui, |ui| {
        let size = ui.available_size();
        let (resp, painter) = mat_canvas(ui, theme, size);
        let rect = resp.rect;
        let area = rect.shrink2(vec2(rect.width() * 0.22, 64.0));
        piece(&painter, theme, area, session);
        rulers(&painter, theme, rect);
        let caption = rect.translate(vec2(22.0, 20.0));
        canvas_label(&painter, theme, caption, "PATRÓN — Delantero");
        let bar = Rect::from_min_max(
            pos2(rect.left(), rect.top() + 28.0),
            pos2(rect.right() - PAD, rect.top() + 54.0),
        );
        let layout = egui::Layout::right_to_left(egui::Align::Min);
        ui.scope_builder(egui::UiBuilder::new().max_rect(bar).layout(layout), |ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            button_secondary(ui, theme, "cm");
            button_secondary(ui, theme, "100%");
        });
    });
}

/// Centimetre rulers along the top and left edges, numbered every fifth tick.
fn rulers(p: &Painter, theme: &Theme, rect: Rect) {
    let top = Rect::from_min_max(rect.left_top(), pos2(rect.right(), rect.top() + 20.0));
    let side = Rect::from_min_max(rect.left_top(), pos2(rect.left() + 20.0, rect.bottom()));
    let (edge, tick) = (Stroke::new(1.0, theme.line), Stroke::new(1.0, theme.muted));
    for band in [top, side] {
        p.rect_filled(band, 0.0, theme.panel);
    }
    p.line_segment([top.left_bottom(), top.right_bottom()], edge);
    p.line_segment([side.right_top(), side.right_bottom()], edge);
    for i in 0..(rect.width() as usize).saturating_sub(20) / 12 {
        let x = rect.left() + 20.0 + i as f32 * 12.0;
        let far = if i.is_multiple_of(5) { 8.0 } else { 20.0 };
        let (a, b) = (pos2(x, rect.top() + 14.0), pos2(x, rect.top() + far));
        p.line_segment([a, b], tick);
        if i.is_multiple_of(5) {
            let at = pos2(x + 2.0, rect.top() + 1.0);
            let font = FontId::monospace(9.0);
            let mark = (i / 5 * 10).to_string();
            p.text(at, Align2::LEFT_TOP, mark, font, theme.muted);
        }
    }
    for i in 0..(rect.height() as usize).saturating_sub(20) / 12 {
        let y = rect.top() + 20.0 + i as f32 * 12.0;
        let far = if i.is_multiple_of(5) { 8.0 } else { 20.0 };
        let (a, b) = (pos2(rect.left() + 14.0, y), pos2(rect.left() + far, y));
        p.line_segment([a, b], tick);
    }
}

/// The selected piece: paper, outline, control points, guide and dimension.
fn piece(p: &Painter, theme: &Theme, area: Rect, session: &Session) {
    let contour = session.contour();
    let map = fit(contour, area);
    let pts: Vec<Pos2> = contour.iter().map(|&q| map(q)).collect();
    let outline = Stroke::new(1.5, theme.outline);
    p.add(Shape::convex_polygon(pts.clone(), theme.paper, outline));
    dimensions(p, theme, Rect::from_points(&pts));
    let alert = Stroke::new(1.0, theme.alert);
    for (i, q) in pts.iter().step_by((pts.len() / POINTS).max(1)).enumerate() {
        if i == GRABBED {
            let handle = *q + vec2(38.0, 26.0);
            p.line_segment([*q, handle], alert);
            p.circle_stroke(handle, 3.0, alert);
            p.circle_filled(*q, 5.0, theme.alert);
        } else {
            p.circle_filled(*q, 3.0, theme.accent);
        }
    }
}

/// The hip guide across the piece and the side-seam dimension beside it.
fn dimensions(p: &Painter, theme: &Theme, bbox: Rect) {
    let stroke = Stroke::new(1.0, theme.measure);
    let hip = bbox.top() + bbox.height() * 0.45;
    let guide = [pos2(bbox.left() - 24.0, hip), pos2(bbox.right() + 8.0, hip)];
    p.extend(Shape::dashed_line(&guide, stroke, 3.0, 4.0));
    let at = pos2(bbox.right() + 14.0, hip);
    let font = FontId::monospace(10.0);
    p.text(at, Align2::LEFT_CENTER, "cadera", font, theme.measure);
    let x = bbox.right() + 34.0;
    p.line_segment([pos2(x, hip), pos2(x, bbox.bottom())], stroke);
    for y in [hip, bbox.bottom()] {
        p.line_segment([pos2(x - 4.0, y), pos2(x + 4.0, y)], stroke);
    }
    let at = pos2(x + 8.0, f32::midpoint(hip, bbox.bottom()));
    let font = FontId::monospace(11.0);
    p.text(at, Align2::LEFT_CENTER, "40.0", font, theme.measure);
}

/// Maps the contour, metres with y up, into a box on the mat, y down.
fn fit(contour: &[[f64; 2]], area: Rect) -> impl Fn([f64; 2]) -> Pos2 {
    let edge = |k: usize, f: fn(f64, f64) -> f64| contour.iter().map(|q| q[k]).fold(f64::NAN, f);
    let (x, y) = (edge(0, f64::min), edge(1, f64::min));
    let (w, h) = (edge(0, f64::max) - x, edge(1, f64::max) - y);
    let scale = f64::from(area.width().min(area.height())) / w.max(h).max(1.0e-6);
    let centre = area.center();
    move |q| {
        let (u, v) = ((q[0] - x - w / 2.0) * scale, (q[1] - y - h / 2.0) * scale);
        pos2(centre.x + u as f32, centre.y - v as f32)
    }
}

fn glyph(p: &Painter, slot: Rect, color: Color32, icon: &str) {
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
