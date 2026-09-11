use eframe::egui::{self, Color32, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};
use toile_engine::body::BodyModel;

use super::State;
use crate::theme::Theme;
use crate::widgets::{list_row_icon, section};

/// The library column: the model switch, then the still-static mockup for
/// standard tables and saved personas (a persistent persona library on disk
/// is a follow-up).
pub fn panel(ui: &mut egui::Ui, theme: &Theme, st: &mut State) {
    section(ui, theme, "Modelo");
    let dummy = list_row_icon(
        ui,
        theme,
        "Maniquí de sastre",
        st.model == BodyModel::TailorDummy,
        person_icon,
    );
    let anny = list_row_icon(
        ui,
        theme,
        "Cuerpo Anny",
        st.model == BodyModel::Anny,
        person_icon,
    );
    let chosen = if dummy.clicked() {
        Some(BodyModel::TailorDummy)
    } else if anny.clicked() {
        Some(BodyModel::Anny)
    } else {
        None
    };
    if let Some(model) = chosen
        && model != st.model
    {
        st.model = model;
        st.dirty = true;
    }

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
