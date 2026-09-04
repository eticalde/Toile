use eframe::egui::{self, Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};

use crate::tabs::{Workspace, left_panel, right_panel};
use crate::theme::Theme;
use crate::widgets::{
    PAD, button_primary, button_secondary, canvas_label, field_row, list_row_icon, mat_canvas,
    rule, section, section_with,
};

/// The mockup draws the figure in a 300 × 640 box whose centreline is x = 120;
/// every coordinate below is in that space and `Art` maps it to the canvas.
const ART_W: f32 = 300.0;
const ART_H: f32 = 640.0;
const AXIS: f32 = 120.0;

/// Half-width of the body at each height, shoulders down to the crotch.
const TORSO: [(f32, f32); 8] = [
    (94.0, 48.0),
    (104.0, 62.0),
    (150.0, 56.0),
    (196.0, 45.0),
    (230.0, 42.0),
    (272.0, 53.0),
    (310.0, 60.0),
    (345.0, 57.0),
];

/// Height, then half-width to the outer and to the inner edge of one leg.
const LEG: [(f32, f32, f32); 5] = [
    (345.0, 57.0, 0.0),
    (400.0, 53.0, 9.0),
    (470.0, 45.0, 15.0),
    (534.0, 41.0, 18.0),
    (600.0, 40.0, 19.0),
];

const GUIDES: [(f32, &str); 3] = [(230.0, "cintura"), (310.0, "cadera"), (470.0, "rodilla")];
const VIEWS: [&str; 4] = ["Frente", "Lado", "Espalda", "Libre"];
const CONTOURS: [(&str, &str); 5] = [
    ("Cintura", "84.0"),
    ("Cadera", "98.0"),
    ("Muslo", "58.0"),
    ("Rodilla", "40.0"),
    ("Tobillo", "24.0"),
];
const LENGTHS: [(&str, &str); 4] = [
    ("Tiro", "27.0"),
    ("Largo lateral", "104.0"),
    ("Entrepierna", "78.0"),
    ("Altura de cadera", "20.0"),
];

pub fn show(ui: &mut egui::Ui, w: &mut Workspace<'_>) {
    let theme = w.theme;
    left_panel(ui, theme, |ui| library(ui, theme));
    right_panel(ui, theme, |ui| measures(ui, theme));
    centre(ui, theme);
}

// ── panels ────────────────────────────────────────────────────────────────

fn library(ui: &mut egui::Ui, theme: &Theme) {
    section(ui, theme, "Tablas estándar");
    for name in ["Talla 38 · ES", "Talla M · ISO 8559"] {
        list_row_icon(ui, theme, name, false, table_icon);
    }
    section(ui, theme, "Personas");
    for (name, selected) in [("Etienne", true), ("Ana", false)] {
        list_row_icon(ui, theme, name, selected, person_icon);
    }
    list_row_icon(ui, theme, "Nueva persona", false, plus_icon);
}

fn measures(ui: &mut egui::Ui, theme: &Theme) {
    section_with(ui, theme, "Medidas · Etienne", "cm");
    section(ui, theme, "Contornos");
    for (label, value) in CONTOURS {
        field_row(ui, theme, label, value, "cm");
    }
    section(ui, theme, "Largos");
    for (label, value) in LENGTHS {
        field_row(ui, theme, label, value, "cm");
    }
    section(ui, theme, "Cuerpo");
    field_row(ui, theme, "Estatura", "178", "cm");
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        actions(ui, theme);
    });
}

/// Pinned to the foot of the inspector, above a rule.
fn actions(ui: &mut egui::Ui, theme: &Theme) {
    ui.add_space(PAD);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.add_space(PAD);
        button_primary(ui, theme, "Usar en Probador");
        button_secondary(ui, theme, "Duplicar");
    });
    ui.add_space(PAD);
    rule(ui, theme);
}

// ── canvas ────────────────────────────────────────────────────────────────

fn centre(ui: &mut egui::Ui, theme: &Theme) {
    egui::CentralPanel::no_frame().show(ui, |ui| {
        let size = ui.available_size();
        let (resp, painter) = mat_canvas(ui, theme, size);
        mannequin(&painter, theme, resp.rect);
        canvas_label(
            &painter,
            theme,
            resp.rect,
            "3D — arrastra para orbitar · rueda para zoom",
        );
        views(ui, theme, resp.rect);
    });
}

fn views(ui: &mut egui::Ui, theme: &Theme, rect: Rect) {
    let bar = Rect::from_min_max(
        pos2(rect.left(), rect.top() + 8.0),
        pos2(rect.right() - 12.0, rect.top() + 34.0),
    );
    let builder = egui::UiBuilder::new()
        .max_rect(bar)
        .layout(egui::Layout::right_to_left(egui::Align::Min));
    ui.scope_builder(builder, |ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        for name in VIEWS.iter().rev() {
            if *name == VIEWS[0] {
                button_primary(ui, theme, name);
            } else {
                button_secondary(ui, theme, name);
            }
        }
    });
}

// ── the mannequin ─────────────────────────────────────────────────────────

/// Places the mockup's drawing space in the middle of the canvas.
struct Art {
    origin: Pos2,
    scale: f32,
}

impl Art {
    fn fit(rect: Rect) -> Self {
        let scale = ((rect.height() - 48.0) / ART_H).clamp(0.35, 1.0);
        let size = vec2(ART_W, ART_H) * scale;
        Self {
            origin: rect.center() - size / 2.0,
            scale,
        }
    }

    fn at(&self, x: f32, y: f32) -> Pos2 {
        self.origin + vec2(x, y) * self.scale
    }

    fn path(&self, pts: &[(f32, f32)]) -> Vec<Pos2> {
        pts.iter().map(|&(x, y)| self.at(x, y)).collect()
    }
}

fn mannequin(painter: &Painter, theme: &Theme, rect: Rect) {
    let art = Art::fit(rect);
    let fill = theme.avatar_color().gamma_multiply(0.6);
    let edge = Stroke::new(1.4, theme.outline.gamma_multiply(0.85));
    let soft = Stroke::new(1.2, theme.outline.gamma_multiply(0.7));

    let head = art.at(AXIS, 44.0);
    let radius = vec2(26.0, 30.0) * art.scale;
    painter.add(Shape::ellipse_filled(head, radius, fill));
    painter.add(Shape::ellipse_stroke(head, radius, soft));
    let neck = art.path(&[(110.0, 70.0), (130.0, 70.0), (132.0, 98.0), (108.0, 98.0)]);
    painter.add(Shape::convex_polygon(neck, fill, soft));

    for band in bands() {
        painter.add(Shape::convex_polygon(art.path(&band), fill, Stroke::NONE));
    }
    painter.add(Shape::closed_line(art.path(&contour()), edge));
    painter.extend(Shape::dashed_line(
        &[art.at(AXIS, 98.0), art.at(AXIS, 600.0)],
        Stroke::new(1.0, theme.outline.gamma_multiply(0.18)),
        2.0,
        5.0,
    ));
    guides(painter, theme, &art);
}

/// The filled body, sliced into trapezoids: egui fills a path by fanning from
/// its first point, so only convex pieces come out whole.
fn bands() -> Vec<Vec<(f32, f32)>> {
    let mut out = Vec::new();
    for pair in TORSO.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        out.push(vec![
            (AXIS - a.1, a.0),
            (AXIS + a.1, a.0),
            (AXIS + b.1, b.0),
            (AXIS - b.1, b.0),
        ]);
    }
    for pair in LEG.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        out.push(vec![
            (AXIS - a.1, a.0),
            (AXIS - a.2, a.0),
            (AXIS - b.2, b.0),
            (AXIS - b.1, b.0),
        ]);
        out.push(vec![
            (AXIS + a.2, a.0),
            (AXIS + a.1, a.0),
            (AXIS + b.1, b.0),
            (AXIS + b.2, b.0),
        ]);
    }
    out
}

/// The outer silhouette, clockwise from the left shoulder.
fn contour() -> Vec<(f32, f32)> {
    let mut p = vec![(AXIS - TORSO[0].1, TORSO[0].0)];
    p.extend(TORSO.iter().map(|&(y, h)| (AXIS + h, y)));
    p.extend(LEG.iter().skip(1).map(|&(y, o, _)| (AXIS + o, y)));
    p.extend(LEG.iter().rev().map(|&(y, _, i)| (AXIS + i, y)));
    p.extend(LEG.iter().skip(1).map(|&(y, _, i)| (AXIS - i, y)));
    p.extend(LEG.iter().rev().map(|&(y, o, _)| (AXIS - o, y)));
    p.extend(
        TORSO
            .iter()
            .skip(1)
            .rev()
            .skip(1)
            .map(|&(y, h)| (AXIS - h, y)),
    );
    p
}

fn guides(painter: &Painter, theme: &Theme, art: &Art) {
    let stroke = Stroke::new(1.0, theme.measure);
    for (y, name) in GUIDES {
        painter.extend(Shape::dashed_line(
            &[art.at(20.0, y), art.at(220.0, y)],
            stroke,
            3.0,
            4.0,
        ));
        painter.text(
            art.at(226.0, y),
            Align2::LEFT_CENTER,
            name,
            FontId::monospace(10.0),
            theme.measure,
        );
    }
}

// ── glyphs ────────────────────────────────────────────────────────────────

fn table_icon(painter: &Painter, r: Rect, color: Color32) {
    let stroke = Stroke::new(1.4, color);
    let b = r.shrink2(vec2(2.0, 3.0));
    painter.rect_stroke(b, 1.0, stroke, egui::StrokeKind::Inside);
    let (split, column) = (b.top() + b.height() * 0.4, b.left() + b.width() * 0.34);
    painter.line_segment([pos2(b.left(), split), pos2(b.right(), split)], stroke);
    painter.line_segment([pos2(column, b.top()), pos2(column, b.bottom())], stroke);
}

fn person_icon(painter: &Painter, r: Rect, color: Color32) {
    let stroke = Stroke::new(1.4, color);
    let c = r.center();
    painter.circle_stroke(c - vec2(0.0, 3.0), 2.6, stroke);
    let shoulders: Vec<Pos2> = [-5.0_f32, -3.4, -1.6, 0.0, 1.6, 3.4, 5.0]
        .iter()
        .map(|&x| c + vec2(x, 6.0 - (25.0 - x * x).sqrt() * 0.9))
        .collect();
    painter.add(Shape::line(shoulders, stroke));
}

fn plus_icon(painter: &Painter, r: Rect, color: Color32) {
    let stroke = Stroke::new(1.4, color);
    let c = r.center();
    painter.line_segment([c - vec2(0.0, 5.0), c + vec2(0.0, 5.0)], stroke);
    painter.line_segment([c - vec2(5.0, 0.0), c + vec2(5.0, 0.0)], stroke);
}
