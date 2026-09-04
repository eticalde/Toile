use eframe::egui::{
    self, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Vec2,
    pos2, vec2,
};

use crate::tabs::{Workspace, left_panel, right_panel};
use crate::theme::Theme;
use crate::widgets::{
    CORNER, PAD, canvas_label, field_row, footer_note, list_row_icon, mat_canvas, section,
};

const FABRICS: [(&str, bool); 4] = [
    ("Algodón popelina", true),
    ("Denim 12 oz", false),
    ("Jersey", false),
    ("Seda", false),
];
const STIFFNESS: [&str; 3] = ["Baja", "Media", "Alta"];
const CAPTION: &str = "MUESTRA — 30 × 30 cm sobre esfera · misma física que el Probador";
const NOTE: &str = "Peso y elasticidad son lo que el solver lee; la rigidez fija el bending. \
                    Un preset es la tupla completa.";

pub fn show(ui: &mut egui::Ui, w: &mut Workspace<'_>) {
    let theme = w.theme;
    left_panel(ui, theme, |ui| library(ui, theme));
    right_panel(ui, theme, |ui| inspector(ui, theme));
    centre(ui, theme);
}

// ── panels ────────────────────────────────────────────────────────────────

fn library(ui: &mut egui::Ui, theme: &Theme) {
    section(ui, theme, "Telas");
    for (name, selected) in FABRICS {
        list_row_icon(ui, theme, name, selected, swatch_icon);
    }
    list_row_icon(ui, theme, "Nueva tela", false, plus_icon);
}

fn inspector(ui: &mut egui::Ui, theme: &Theme) {
    section(ui, theme, "Propiedades · Algodón popelina");
    field_row(ui, theme, "Peso", "120", "g/m²");
    field_row(ui, theme, "Ancho de rollo", "150", "cm");
    section(ui, theme, "Elasticidad");
    field_row(ui, theme, "Urdimbre", "2", "%");
    field_row(ui, theme, "Trama", "3", "%");
    section(ui, theme, "Rigidez de doblado");
    stiffness(ui, theme);
    section(ui, theme, "Color");
    colour(ui, theme);
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        footer_note(ui, theme, NOTE);
    });
}

/// The bending preset: three exclusive options sharing the panel width.
fn stiffness(ui: &mut egui::Ui, theme: &Theme) {
    let width = (ui.available_width() - 2.0 * PAD - 8.0) / 3.0;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.add_space(PAD);
        for (i, name) in STIFFNESS.iter().enumerate() {
            let (rect, _) = ui.allocate_exact_size(vec2(width, 24.0), Sense::hover());
            let active = i == 0;
            let p = ui.painter();
            if active {
                p.rect_filled(rect, CORNER, theme.accent.gamma_multiply(0.16));
            }
            let edge = if active { theme.accent } else { theme.line };
            p.rect_stroke(rect, CORNER, Stroke::new(1.0, edge), StrokeKind::Inside);
            let ink = if active { theme.ink } else { theme.ink_soft };
            let font = FontId::proportional(11.0);
            p.text(rect.center(), Align2::CENTER_CENTER, *name, font, ink);
        }
    });
    ui.add_space(6.0);
}

/// The cloth colour, and the hex of exactly what the swatch paints.
fn colour(ui: &mut egui::Ui, theme: &Theme) {
    let fill = theme.cloth_color();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        ui.add_space(PAD);
        let (rect, _) = ui.allocate_exact_size(vec2(36.0, 24.0), Sense::hover());
        let line = Stroke::new(1.0, theme.line);
        ui.painter()
            .rect(rect, CORNER, fill, line, StrokeKind::Inside);
        let hex = format!("#{:02x}{:02x}{:02x}", fill.r(), fill.g(), fill.b());
        let text = egui::RichText::new(hex).monospace().size(12.0);
        ui.label(text.color(theme.ink));
    });
}

// ── canvas ────────────────────────────────────────────────────────────────

fn centre(ui: &mut egui::Ui, theme: &Theme) {
    egui::CentralPanel::no_frame().show(ui, |ui| {
        let size = ui.available_size();
        let (resp, painter) = mat_canvas(ui, theme, size);
        sample(&painter, theme, resp.rect);
        canvas_label(&painter, theme, resp.rect, CAPTION);
    });
}

/// The mockup draws the swatch in a 420 × 360 box; every coordinate below is
/// in that space and `Art` maps it onto the canvas.
const ART: Vec2 = vec2(420.0, 360.0);
const BALL: (f32, f32, f32) = (210.0, 230.0, 96.0);

/// The hem corners the two boundaries of the drape start from.
const LEFT_HEM: (f32, f32) = (90.0, 300.0);
const RIGHT_HEM: (f32, f32) = (330.0, 300.0);

/// Cubic segments, `(cx1, cy1, cx2, cy2, x, y)`, running left to right over
/// the selvage and right to left over the scalloped hem.
const SELVAGE: [[f32; 6]; 3] = [
    [102.0, 240.0, 108.0, 180.0, 112.0, 120.0],
    [140.0, 108.0, 280.0, 108.0, 308.0, 120.0],
    [312.0, 180.0, 318.0, 240.0, 330.0, 300.0],
];
const HEM: [[f32; 6]; 7] = [
    [300.0, 306.0, 270.0, 300.0, 262.0, 296.0],
    [262.0, 260.0, 258.0, 220.0, 254.0, 190.0],
    [252.0, 240.0, 250.0, 280.0, 250.0, 300.0],
    [220.0, 306.0, 200.0, 306.0, 170.0, 300.0],
    [170.0, 280.0, 168.0, 240.0, 166.0, 190.0],
    [162.0, 220.0, 158.0, 260.0, 158.0, 296.0],
    [150.0, 300.0, 120.0, 306.0, 90.0, 300.0],
];
/// One crease each, start point first, then its single cubic.
const CREASES: [[f32; 8]; 3] = [
    [140.0, 130.0, 146.0, 190.0, 144.0, 250.0, 138.0, 290.0],
    [280.0, 130.0, 274.0, 190.0, 276.0, 250.0, 282.0, 290.0],
    [210.0, 118.0, 210.0, 131.0, 210.0, 145.0, 210.0, 158.0],
];
const SHADOW_FROM: (f32, f32) = (100.0, 250.0);
const SHADOW: [[f32; 6]; 2] = [
    [130.0, 220.0, 160.0, 220.0, 200.0, 210.0],
    [240.0, 200.0, 270.0, 220.0, 310.0, 250.0],
];
/// Points per cubic when flattening, and columns across the filled drape.
const STEPS: usize = 12;
const COLUMNS: usize = 72;

struct Art {
    origin: Pos2,
    scale: f32,
}

impl Art {
    fn fit(rect: Rect) -> Self {
        let scale = ((rect.height() - 64.0) / ART.y).clamp(0.4, 1.3);
        Self {
            origin: rect.center() - ART * scale / 2.0,
            scale,
        }
    }

    fn at(&self, x: f32, y: f32) -> Pos2 {
        self.origin + vec2(x, y) * self.scale
    }
}

fn sample(p: &Painter, theme: &Theme, rect: Rect) {
    let art = Art::fit(rect);
    let (cx, cy, r) = BALL;
    let ball = art.at(cx, cy);
    p.circle_filled(ball, r * art.scale, theme.avatar_color());
    let rim = Stroke::new(1.0, theme.outline.gamma_multiply(0.5));
    p.circle_stroke(ball, r * art.scale, rim);

    let selvage = flatten(LEFT_HEM, &SELVAGE);
    let hem = flatten(RIGHT_HEM, &HEM);
    drape(p, &art, &selvage, &hem, theme.cloth_color());
    let mut edge: Vec<Pos2> = selvage.iter().map(|&(x, y)| art.at(x, y)).collect();
    edge.extend(hem.iter().map(|&(x, y)| art.at(x, y)));
    let seam = Stroke::new(1.0, theme.outline.gamma_multiply(0.45));
    p.add(Shape::closed_line(edge, seam));
    folds(p, theme, &art);
}

/// The cloth body, sliced into columns: egui fills a path by fanning from its
/// first point, so only convex pieces come out whole.
fn drape(p: &Painter, art: &Art, selvage: &[(f32, f32)], hem: &[(f32, f32)], fill: Color32) {
    let mut floor = hem.to_vec();
    floor.reverse();
    let (x0, x1) = (LEFT_HEM.0, RIGHT_HEM.0);
    let column = |i: usize| {
        let x = x0 + (x1 - x0) * i as f32 / COLUMNS as f32;
        (x, height(selvage, x), height(&floor, x))
    };
    for i in 1..=COLUMNS {
        let (xa, ta, ba) = column(i - 1);
        let (xb, tb, bb) = column(i);
        let quad = vec![
            art.at(xa, ta),
            art.at(xb, tb),
            art.at(xb, bb),
            art.at(xa, ba),
        ];
        p.add(Shape::convex_polygon(quad, fill, Stroke::NONE));
    }
}

/// Where an x-monotone boundary sits at `x`.
fn height(chain: &[(f32, f32)], x: f32) -> f32 {
    let i = chain.partition_point(|q| q.0 < x).clamp(1, chain.len() - 1);
    let (a, b) = (chain[i - 1], chain[i]);
    let span = b.0 - a.0;
    if span.abs() < f32::EPSILON {
        a.1
    } else {
        a.1 + (b.1 - a.1) * (x - a.0) / span
    }
}

/// The creases and the shadow the ball throws across the cloth.
fn folds(p: &Painter, theme: &Theme, art: &Art) {
    let stroke = Stroke::new(1.0, theme.mat.gamma_multiply(0.35));
    let draw = |chain: &[(f32, f32)]| {
        let path: Vec<Pos2> = chain.iter().map(|&(x, y)| art.at(x, y)).collect();
        p.add(Shape::line(path, stroke));
    };
    for c in CREASES {
        draw(&flatten(
            (c[0], c[1]),
            &[[c[2], c[3], c[4], c[5], c[6], c[7]]],
        ));
    }
    draw(&flatten(SHADOW_FROM, &SHADOW));
}

fn flatten(start: (f32, f32), segments: &[[f32; 6]]) -> Vec<(f32, f32)> {
    let mut out = vec![start];
    let mut from = start;
    for s in segments {
        for i in 1..=STEPS {
            out.push(cubic(from, *s, i as f32 / STEPS as f32));
        }
        from = (s[4], s[5]);
    }
    out
}

fn cubic(from: (f32, f32), s: [f32; 6], t: f32) -> (f32, f32) {
    let u = 1.0 - t;
    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    (
        a * from.0 + b * s[0] + c * s[2] + d * s[4],
        a * from.1 + b * s[1] + c * s[3] + d * s[5],
    )
}

// ── glyphs ────────────────────────────────────────────────────────────────

/// A bolt of cloth: the square of the sample and the wave of the weave.
fn swatch_icon(p: &Painter, r: Rect, color: Color32) {
    let stroke = Stroke::new(1.3, color);
    let b = r.shrink(2.0);
    p.rect_stroke(b, 1.0, stroke, StrokeKind::Inside);
    let wave: Vec<Pos2> = (0..=8)
        .map(|i| {
            let t = i as f32 / 8.0;
            let phase = t * std::f32::consts::TAU;
            pos2(b.left() + b.width() * t, b.center().y + 2.0 * phase.sin())
        })
        .collect();
    p.add(Shape::line(wave, stroke));
}

fn plus_icon(p: &Painter, r: Rect, color: Color32) {
    let stroke = Stroke::new(1.4, color);
    let c = r.center();
    p.line_segment([c - vec2(0.0, 5.0), c + vec2(0.0, 5.0)], stroke);
    p.line_segment([c - vec2(5.0, 0.0), c + vec2(5.0, 0.0)], stroke);
}
