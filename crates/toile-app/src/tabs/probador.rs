use eframe::egui::{
    self, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Shape, Stroke, Vec2, pos2, vec2,
};
use eframe::egui_wgpu::RenderState;
use toile_engine::session::Session;

use crate::pattern;
use crate::tabs::{Workspace, right_panel};
use crate::theme::Theme;
use crate::viewport::Viewport;
use crate::widgets::{PAD, button_icon, field_row, footer_note, section, section_with, select};

/// Gap between the 2D and 3D halves, in points.
const SPLIT_GAP: f32 = 12.0;
const SUBBAR_H: f32 = 44.0;
const SEAM_H: f32 = 28.0;
const MARK: f32 = 12.0;

/// Each seam, with the length its two sides come out to and whether they meet.
const SEAMS: [(&str, &str, &str, bool); 5] = [
    ("Lateral izq.", "104.0", "104.0", true),
    ("Lateral der.", "104.0", "104.0", true),
    ("Entrepierna", "78.0", "78.0", true),
    ("Tiro", "27.0", "29.5", false),
    ("Pretina", "84.0", "84.0", true),
];
const MISMATCH: &str = "tiro: los largos difieren 2.5 cm";
const NOTE: &str = "Editar un punto en 2D re-drapea sin resetear la simulación.";

/// The tab's own state: a GPU viewport and the drag in progress.
pub struct State {
    rs: RenderState,
    viewport: Viewport,
    /// The node being dragged on the 2D half, while one is.
    drag: Option<pattern::Drag>,
}

impl State {
    pub fn new(rs: RenderState, theme: &Theme, session: &Session) -> Self {
        let viewport = Viewport::new(
            &rs,
            theme,
            session.n_vertices(),
            session.triangles(),
            session.avatar_radius(),
        );
        Self {
            rs,
            viewport,
            drag: None,
        }
    }
}

pub fn show(ui: &mut egui::Ui, w: &mut Workspace<'_>) {
    let theme = w.theme;
    sub_bar(ui, theme);
    right_panel(ui, theme, |ui| inspector(ui, theme));
    egui::CentralPanel::no_frame().show(ui, |ui| {
        let full = ui.available_size();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let half = vec2((full.x - SPLIT_GAP) / 2.0, full.y);
            let st = &mut *w.probador;
            pattern::show(ui, half, theme, w.session, &mut st.drag);
            gutter(ui, theme, full.y);
            st.viewport.show(ui, half, &st.rs, theme, w.session);
        });
    });
}

// ── bars and panels ───────────────────────────────────────────────────────

fn sub_bar(ui: &mut egui::Ui, theme: &Theme) {
    egui::Panel::top("probador-subbar")
        .exact_size(SUBBAR_H)
        .frame(
            egui::Frame::new()
                .fill(theme.panel)
                .inner_margin(egui::Margin::symmetric(16, 0)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                select(ui, theme, "maniquí", "Etienne", 150.0);
                select(ui, theme, "producto", "Pantalón base", 170.0);
                select(ui, theme, "tela", "Algodón popelina", 170.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    button_icon(ui, theme, "Reiniciar", false, reset_icon);
                    button_icon(ui, theme, "Pausar", false, pause_icon);
                    button_icon(ui, theme, "Simular", true, play_icon);
                });
            });
        });
}

/// The seam table, and what the selected piece is made of.
fn inspector(ui: &mut egui::Ui, theme: &Theme) {
    section_with(ui, theme, "Costuras", "5");
    for seam in SEAMS {
        seam_row(ui, theme, seam);
    }
    mismatch(ui, theme);
    section(ui, theme, "Pieza seleccionada");
    field_row(ui, theme, "Delantero · tela", "Algodón", "");
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        footer_note(ui, theme, NOTE);
    });
}

/// Seam name, the two lengths, and the mark saying whether they close.
fn seam_row(ui: &mut egui::Ui, theme: &Theme, seam: (&str, &str, &str, bool)) {
    let (name, left, right, ok) = seam;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), SEAM_H), Sense::hover());
    let p = ui.painter();
    p.text(
        rect.left_center() + vec2(PAD, 0.0),
        Align2::LEFT_CENTER,
        name,
        FontId::proportional(12.0),
        theme.ink_soft,
    );
    let slot = Rect::from_center_size(
        rect.right_center() - vec2(PAD + MARK / 2.0, 0.0),
        Vec2::splat(MARK),
    );
    let mark: fn(&Painter, Rect, Color32) = if ok { check_icon } else { warn_icon };
    let tone = if ok { theme.accent } else { theme.alert };
    mark(p, slot, tone);
    p.text(
        pos2(slot.left() - 8.0, rect.center().y),
        Align2::RIGHT_CENTER,
        format!("{left} / {right}"),
        FontId::monospace(11.0),
        if ok { theme.ink } else { theme.alert },
    );
}

/// Says in words what the warning mark on the seam row only hints at.
fn mismatch(ui: &mut egui::Ui, theme: &Theme) {
    let margin = egui::Margin {
        left: 12,
        right: 12,
        top: 6,
        bottom: 10,
    };
    egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        let body = egui::RichText::new(MISMATCH).monospace().size(11.0);
        ui.label(body.color(theme.alert));
    });
}

/// The seam between the two halves of the split, ruled on both sides.
fn gutter(ui: &mut egui::Ui, theme: &Theme, height: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(SPLIT_GAP, height), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 0.0, theme.panel);
    let stroke = Stroke::new(1.0, theme.line);
    p.line_segment([rect.left_top(), rect.left_bottom()], stroke);
    p.line_segment([rect.right_top(), rect.right_bottom()], stroke);
}

// ── glyphs ────────────────────────────────────────────────────────────────

fn play_icon(p: &Painter, r: Rect, color: Color32) {
    let b = r.shrink(1.0);
    let tip = pos2(b.right(), b.center().y);
    let points = vec![b.left_top(), tip, b.left_bottom()];
    p.add(Shape::convex_polygon(points, color, Stroke::NONE));
}

fn pause_icon(p: &Painter, r: Rect, color: Color32) {
    let b = r.shrink2(vec2(2.5, 1.0));
    let stroke = Stroke::new(1.6, color);
    p.line_segment([b.left_top(), b.left_bottom()], stroke);
    p.line_segment([b.right_top(), b.right_bottom()], stroke);
}

/// An almost closed circle, with the corner mark of an arrowhead at its start.
fn reset_icon(p: &Painter, r: Rect, color: Color32) {
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

fn check_icon(p: &Painter, r: Rect, color: Color32) {
    let at = |x: f32, y: f32| r.left_top() + vec2(x, y) * r.width() / 12.0;
    let tick = vec![at(2.5, 6.5), at(5.0, 9.5), at(9.5, 3.0)];
    p.add(Shape::line(tick, Stroke::new(1.5, color)));
}

fn warn_icon(p: &Painter, r: Rect, color: Color32) {
    let at = |x: f32, y: f32| r.left_top() + vec2(x, y) * r.width() / 12.0;
    let stroke = Stroke::new(1.2, color);
    let body = vec![at(6.0, 1.5), at(11.0, 10.5), at(1.0, 10.5)];
    p.add(Shape::closed_line(body, stroke));
    p.line_segment([at(6.0, 5.0), at(6.0, 8.5)], stroke);
}
