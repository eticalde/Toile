mod measures;

use eframe::egui::{self, Color32, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};
use eframe::egui_wgpu::RenderState;
use toile_engine::body;
use toile_engine::draft::MeasureSet;

use crate::tabs::{Workspace, left_panel, right_panel};
use crate::theme::Theme;
use crate::viewport::BodyView;
use crate::widgets::{list_row_icon, section};

/// The mannequins tab: an editable set of body measurements and the procedural
/// 3D dummy they loft, regenerated live as a value changes.
///
/// The measure set is tab-local for now; wiring it to the document's mannequins
/// and a persistent persona library is a follow-up. Nothing here touches the
/// sphere avatar or the Probador's session.
pub struct State {
    rs: RenderState,
    view: BodyView,
    /// The measurements driving the body, keyed by catalogue name (the user's
    /// Spanish data), edited in the inspector.
    measures: MeasureSet,
    /// The catalogue name being written and the text so far, kept off the
    /// measure set until it parses, so a half-typed value still paints.
    editing: Option<(String, String)>,
    /// A value changed this frame; rebuild the mesh before the next paint.
    dirty: bool,
}

impl State {
    pub fn new(rs: RenderState, theme: &Theme) -> Self {
        let measures = body::default_measures();
        let mut view = BodyView::new(&rs, theme);
        view.set_mesh(&rs, &body::body_from_measures(&measures));
        Self {
            rs,
            view,
            measures,
            editing: None,
            dirty: false,
        }
    }
}

pub fn show(ui: &mut egui::Ui, w: &mut Workspace<'_>) {
    let theme = w.theme;
    left_panel(ui, theme, |ui| library(ui, theme));
    let st = &mut *w.maniquies;
    right_panel(ui, theme, |ui| measures::panel(ui, theme, st));
    egui::CentralPanel::no_frame().show(ui, |ui| {
        let size = ui.available_size();
        // A measurement edit only marks the state dirty; the rebuild happens
        // here, once, right before the frame that shows it. The loft is
        // microseconds, so there is no thread and no generation gating.
        if st.dirty {
            let mesh = body::body_from_measures(&st.measures);
            st.view.set_mesh(&st.rs, &mesh);
            st.dirty = false;
        }
        st.view.show(ui, size, &st.rs, theme);
    });
}

// ── panels ────────────────────────────────────────────────────────────────

/// The library column, still a static mockup: standard tables and saved
/// personas are a follow-up (a persistent persona library on disk).
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
