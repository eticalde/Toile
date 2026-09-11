mod measures;

use eframe::egui::{self, Color32, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};
use eframe::egui_wgpu::RenderState;
use toile_engine::body::{self, BodyModel};
use toile_engine::draft::{MeasureSet, Station};

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
    /// Which mesh producer is showing. Switching rebuilds the mesh; the
    /// tailor's dummy reads `measures`; Anny does not yet (see
    /// `BodyModel::Anny`'s doc).
    model: BodyModel,
    /// The catalogue name whose region the body lights: the row last hovered
    /// or handled, kept lit after the pointer leaves it so the person can look
    /// from the slider to the body.
    highlight: Option<String>,
    /// The station mask the body was last coloured with, so a frame that
    /// changes nothing uploads nothing.
    lit: u32,
    /// A value changed this frame; rebuild the mesh before the next paint.
    dirty: bool,
}

impl State {
    /// `anny_body` seeds the model from the saved preference, so the tab
    /// reopens on whichever body the person last looked at.
    pub fn new(rs: RenderState, theme: &Theme, anny_body: bool) -> Self {
        let measures = body::default_measures();
        let model = if anny_body {
            BodyModel::Anny
        } else {
            BodyModel::TailorDummy
        };
        let mut view = BodyView::new(&rs, theme);
        view.set_mesh(&rs, &body::body_from_measures_with(model, &measures), 0);
        Self {
            rs,
            view,
            measures,
            model,
            highlight: None,
            lit: 0,
            dirty: false,
        }
    }

    /// Whether the tab is currently showing the Anny body, for the
    /// preference to remember on exit.
    pub fn uses_anny(&self) -> bool {
        self.model == BodyModel::Anny
    }
}

/// The bit mask of a set of stations, one bit per tag: what the body view
/// colours by.
fn mask_of(stations: &[Station]) -> u32 {
    stations
        .iter()
        .fold(0, |mask, s| mask | (1 << u32::from(s.tag())))
}

pub fn show(ui: &mut egui::Ui, w: &mut Workspace<'_>) {
    let theme = w.theme;
    let st = &mut *w.maniquies;
    left_panel(ui, theme, |ui| library(ui, theme, st));
    right_panel(ui, theme, |ui| measures::panel(ui, theme, st));
    egui::CentralPanel::no_frame().show(ui, |ui| {
        let size = ui.available_size();
        let mask = st
            .highlight
            .as_deref()
            .map_or(0, |name| mask_of(body::stations_for(name)));
        // A measurement edit or a model switch only marks the state dirty;
        // the rebuild happens here, once, right before the frame that shows
        // it. Both producers answer in microseconds, so there is no thread
        // and no generation gating. A change of highlight alone recolours
        // without rebuilding.
        if st.dirty {
            let mesh = body::body_from_measures_with(st.model, &st.measures);
            st.view.set_mesh(&st.rs, &mesh, mask);
            st.dirty = false;
            st.lit = mask;
        } else if mask != st.lit {
            st.view.set_highlight(&st.rs, mask);
            st.lit = mask;
        }
        st.view.show(ui, size, &st.rs, theme);
    });
}

// ── panels ────────────────────────────────────────────────────────────────

/// The library column: the model switch, then the still-static mockup for
/// standard tables and saved personas (a persistent persona library on disk
/// is a follow-up).
fn library(ui: &mut egui::Ui, theme: &Theme, st: &mut State) {
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
