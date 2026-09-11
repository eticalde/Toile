mod library;
mod measures;
mod phenotype;

use eframe::egui;
use eframe::egui_wgpu::RenderState;
use toile_engine::body::{self, AnnySolve, BodyModel, Phenotype};
use toile_engine::draft::{BodyMesh, MeasureSet, Station};

use crate::tabs::{Workspace, left_panel, right_panel};
use crate::theme::Theme;
use crate::viewport::BodyView;

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
    /// tailor's dummy reads `measures`; Anny reads `anny` instead.
    model: BodyModel,
    /// Anny's phenotype controls, edited only while `model` is `Anny`; see
    /// `phenotype::panel`.
    anny: AnnyControls,
    /// The stature the last-built Anny mesh actually measured, in
    /// centimetres — shown next to the height slider since Anny's own
    /// `height` input is not centimetres (see `body::stature_cm`).
    anny_stature_cm: f32,
    /// The last full lever solve against `measures`: the phenotype with its
    /// `height` closed against `estatura`, the 20 lever values that closed
    /// (or came as close as the model allows to) every other row, and each
    /// row's medido/Δ/tope-del-modelo outcome — the *medido* half of the
    /// measures panel's dado/medido/Δ. `None` while the tailor's dummy is
    /// showing: that model has no levers and no such reading, so its rows
    /// keep no medido column at all.
    anny_solved: Option<AnnySolve>,
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

/// The six phenotype controls the person edits, in their own natural units
/// (years for age, Anny's own `[0, 1]` for everything else); converted to a
/// `Phenotype` only when a mesh is built.
struct AnnyControls {
    /// `0.0` male .. `1.0` female.
    sex: f64,
    /// In years; converted with `body::age_param_from_years`.
    age_years: f64,
    /// Build: `0.0` thinnest .. `1.0` heaviest.
    weight: f64,
    /// `0.0` least muscular .. `1.0` most muscular.
    muscle: f64,
    /// Anny's own height parameter, not centimetres — see `anny_stature_cm`.
    height: f64,
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
            height: 0.5,
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
            height: self.height,
            proportions: self.proportions,
        }
    }
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
        let anny = AnnyControls::default();
        let mut view = BodyView::new(&rs, theme);
        let (mesh, anny_solved, anny_stature_cm) = build(model, &measures, &anny);
        view.set_mesh(&rs, &mesh, 0);
        Self {
            rs,
            view,
            measures,
            model,
            anny,
            anny_stature_cm,
            anny_solved,
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

/// Builds the mesh the current model shows.
///
/// The tailor's dummy reads `measures` directly and carries no levers at
/// all, so it hits every girth by construction and `build` reports no
/// solve. Anny instead solves first: [`body::solve_anny`] runs a fixed
/// order of secant solves (`estatura`'s `height` phenotype input, then the
/// lengths that move stature, then the trunk girths in two coupled sweeps,
/// then the independent limbs and head) and hands back the phenotype and
/// lever vector that came closest to `measures`, which is what the mesh is
/// actually lofted from — not the raw, unsolved phenotype `anny` carries.
///
/// Called once per commit (a slider release, a typed value, a model
/// switch), never per drag frame: a full 20-row solve costs low
/// milliseconds, cheap enough to not need a background thread, too much to
/// pay 60 times a second while a slider is merely being dragged.
fn build(
    model: BodyModel,
    measures: &MeasureSet,
    anny: &AnnyControls,
) -> (BodyMesh, Option<AnnySolve>, f32) {
    match model {
        BodyModel::TailorDummy => {
            let mesh = body::body_from_measures_with(
                model,
                measures,
                &anny.to_phenotype(),
                &body::NO_LEVERS,
            );
            let stature = body::stature_cm(&mesh);
            (mesh, None, stature)
        }
        BodyModel::Anny => {
            let solved = body::solve_anny(measures, &anny.to_phenotype());
            let mesh =
                body::body_from_measures_with(model, measures, &solved.phenotype, &solved.levers);
            let stature = body::stature_cm(&mesh);
            (mesh, Some(solved), stature)
        }
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
    left_panel(ui, theme, |ui| library::panel(ui, theme, st));
    right_panel(ui, theme, |ui| {
        phenotype::panel(ui, theme, st);
        measures::panel(ui, theme, st);
    });
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
            let (mesh, anny_solved, stature) = build(st.model, &st.measures, &st.anny);
            st.anny_solved = anny_solved;
            if st.model == BodyModel::Anny {
                st.anny_stature_cm = stature;
            }
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
