mod identity;
mod measures;

use eframe::egui;
use eframe::egui_wgpu::RenderState;
use toile_engine::body::{self, AnnySolve, Phenotype};
use toile_engine::draft::{BodyMesh, MeasureSet, Station};

use crate::tabs::{Body, Workspace, left_panel, right_panel};
use crate::theme::Theme;
use crate::viewport::BodyView;

/// The mannequins tab: the Anny body, its phenotype controls, and an
/// editable set of measurements solved against it live as a value changes.
///
/// Nothing here touches the Probador's session: the body is this tab's own,
/// and the pattern on the table goes on resolving against the mannequin the
/// document names.
pub struct State {
    rs: RenderState,
    view: BodyView,
    /// The mannequin's own name, shown as the measures panel's header.
    name: String,
    /// The measurements driving the body, keyed by catalogue name (the
    /// user's Spanish data), edited in the inspector.
    measures: MeasureSet,
    /// Anny's phenotype controls, edited in `identity::panel`.
    anny: AnnyControls,
    /// The stature the last-built mesh actually measured, in centimetres —
    /// shown next to the phenotype controls, since Anny's own `height`
    /// input is not centimetres (see `body::stature_cm`).
    anny_stature_cm: f32,
    /// The last full lever solve against `measures`: the phenotype with
    /// its `height` closed against `estatura`, the 20 lever values that
    /// closed (or came as close as the model allows to) every other row,
    /// and each row's medido/Δ/tope-del-modelo outcome — the *medido*
    /// half of the measures panel's dado/medido/Δ.
    anny_solved: AnnySolve,
    /// The catalogue name whose region the body lights: the row last hovered
    /// or handled, kept lit after the pointer leaves it so the person can look
    /// from the slider to the body.
    highlight: Option<String>,
    /// The station mask the body was last coloured with, so a frame that
    /// changes nothing uploads nothing.
    lit: u32,
    /// A value changed this frame; rebuild the mesh before the next paint.
    dirty: bool,
    /// The new-mannequin dialog's own fields, while it is open.
    new_dialog: Option<identity::NewManiqui>,
}

/// The five phenotype controls the person edits, in their own natural units
/// (years for age, Anny's own `[0, 1]` for the rest); converted to a
/// `Phenotype` only when a mesh is built. Stature is not among them: it is
/// one of the twenty catalogue measurements, and the solver owns the
/// `height` phenotype input that answers to it.
struct AnnyControls {
    /// `0.0` male .. `1.0` female.
    sex: f64,
    /// In years; converted with `body::age_param_from_years`.
    age_years: f64,
    /// Build: `0.0` thinnest .. `1.0` heaviest.
    weight: f64,
    /// `0.0` least muscular .. `1.0` most muscular.
    muscle: f64,
    /// `0.0` typical proportions .. `1.0` atypical.
    proportions: f64,
}

impl Default for AnnyControls {
    /// A 25-year-old adult at every other neutral midpoint — the same
    /// default `Phenotype::default()` carries, spelled out here because the
    /// person edits years, not Anny's raw age parameter.
    fn default() -> Self {
        Self {
            sex: 0.5,
            age_years: 25.0,
            weight: 0.5,
            muscle: 0.5,
            proportions: 0.5,
        }
    }
}

impl AnnyControls {
    fn to_phenotype(&self) -> Phenotype {
        Phenotype {
            gender: self.sex,
            age: body::age_param_from_years(self.age_years),
            muscle: self.muscle,
            weight: self.weight,
            // `solve_anny`'s stature group always re-derives this from
            // `estatura` before anything else runs (see its own doc), so
            // the seed value here never survives past its first secant
            // guess.
            height: 0.5,
            proportions: self.proportions,
        }
    }
}

impl State {
    pub fn new(rs: RenderState, theme: &Theme) -> Self {
        let name = "Maniquí".to_owned();
        let measures = body::default_measures();
        let anny = AnnyControls::default();
        let mut view = BodyView::new(&rs, theme);
        let (mesh, anny_solved, anny_stature_cm) = build(&measures, &anny);
        view.set_mesh(&rs, &mesh, 0);
        Self {
            rs,
            view,
            name,
            measures,
            anny,
            anny_stature_cm,
            anny_solved,
            highlight: None,
            lit: 0,
            dirty: false,
            new_dialog: None,
        }
    }

    /// The body on the stand, as the status bar has to name it.
    ///
    /// The same two things the measures panel puts at the top of itself, so
    /// the bar cannot come to disagree with the panel above it.
    pub fn body(&self) -> Body<'_> {
        Body {
            name: &self.name,
            measures: self.measures.values.len(),
        }
    }
}

/// Solves the levers against `measures` and lofts the resulting mesh.
///
/// [`body::solve_anny`] runs a fixed order of secant solves (`estatura`'s
/// `height` phenotype input together with the lengths that move it, then
/// the trunk girths in two coupled sweeps, then the independent limbs and
/// head) and hands back the phenotype and lever vector that came closest
/// to `measures`, which is what the mesh is actually lofted from — not the
/// raw, unsolved phenotype `anny` carries.
///
/// Called once per commit (a slider release, a typed value, a new
/// mannequin), never per drag frame: a full 20-row solve costs low
/// milliseconds, cheap enough to not need a background thread, too much to
/// pay 60 times a second while a slider is merely being dragged.
fn build(measures: &MeasureSet, anny: &AnnyControls) -> (BodyMesh, AnnySolve, f32) {
    let solved = body::solve_anny(measures, &anny.to_phenotype());
    let mesh = body::body_mesh(&solved.phenotype, &solved.levers);
    let stature = body::stature_cm(&mesh);
    (mesh, solved, stature)
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
    left_panel(ui, theme, |ui| identity::panel(ui, theme, st));
    right_panel(ui, theme, |ui| measures::panel(ui, theme, st));
    egui::CentralPanel::no_frame().show(ui, |ui| {
        let size = ui.available_size();
        let mask = st
            .highlight
            .as_deref()
            .map_or(0, |name| mask_of(body::stations_for(name)));
        // A measurement edit, a phenotype edit or a new mannequin only marks
        // the state dirty; the rebuild — and the solve — happens here,
        // once, right before the frame that shows it. A change of
        // highlight alone recolours without rebuilding.
        if st.dirty {
            let (mesh, solved, stature) = build(&st.measures, &st.anny);
            st.anny_solved = solved;
            st.anny_stature_cm = stature;
            st.view.set_mesh(&st.rs, &mesh, mask);
            st.dirty = false;
            st.lit = mask;
        } else if mask != st.lit {
            st.view.set_highlight(&st.rs, mask);
            st.lit = mask;
        }
        st.view.show(ui, size, &st.rs, theme);
    });
    identity::dialog(ui.ctx(), theme, st);
}
