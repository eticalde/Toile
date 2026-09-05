pub mod maniquies;
pub mod patronaje;
pub mod probador;
pub mod telas;

use eframe::egui;
use toile_engine::session::Session;

use crate::theme::Theme;
use crate::widgets;

/// Left panel width, in points, from the layout mockups.
const LEFT_W: f32 = 232.0;
const RIGHT_W: f32 = 296.0;

/// The four stages of the pipeline, in the order the top bar shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Maniquies,
    Patronaje,
    Telas,
    Probador,
}

impl Tab {
    pub const ALL: [Self; 4] = [
        Self::Maniquies,
        Self::Patronaje,
        Self::Telas,
        Self::Probador,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Maniquies => "Maniquíes",
            Self::Patronaje => "Patronaje",
            Self::Telas => "Telas",
            Self::Probador => "Probador",
        }
    }

    /// The cells of the status bar, left to right.
    ///
    /// The drafting tab's own state comes along because some of what the bar
    /// has to report is not in the document: an edit the session refused
    /// leaves the drawing untouched and the drape behind, and the bar is the
    /// only place that says so.
    pub fn status(self, session: &Session, patronaje: &patronaje::State) -> Vec<String> {
        let fixed: &[&str] = match self {
            Self::Maniquies => &["maniquí Etienne", "12 medidas", "cm"],
            Self::Patronaje => return patronaje::status(session, patronaje),
            Self::Telas => &["Algodón popelina", "120 g/m²", "4 telas"],
            Self::Probador => {
                let snap = session.snapshot();
                let sim = if snap.converged {
                    "sim dormida (0% CPU)"
                } else {
                    "sim corriendo"
                };
                return vec![
                    format!("substeps {}", snap.substeps),
                    sim.to_owned(),
                    format!("derive {:.1} ms", session.last_derive_ms),
                    "Etienne · Pantalón base · Algodón popelina".to_owned(),
                ];
            }
        };
        fixed.iter().map(|s| (*s).to_owned()).collect()
    }

    pub fn show(self, ui: &mut egui::Ui, w: &mut Workspace<'_>) {
        match self {
            Self::Maniquies => maniquies::show(ui, w),
            Self::Patronaje => patronaje::show(ui, w),
            Self::Telas => telas::show(ui, w),
            Self::Probador => probador::show(ui, w),
        }
    }
}

/// Everything a tab may read or write while it draws.
///
/// One bundle instead of a growing argument list: a tab that later needs the
/// document only reaches deeper into the session, and no signature moves.
pub struct Workspace<'a> {
    pub theme: &'a Theme,
    pub session: &'a mut Session,
    pub patronaje: &'a mut patronaje::State,
    pub probador: &'a mut probador::State,
}

/// Library or tool column, on the left.
pub fn left_panel<R>(ui: &mut egui::Ui, theme: &Theme, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Panel::left("left")
        .exact_size(LEFT_W)
        .resizable(false)
        .frame(egui::Frame::new().fill(theme.panel))
        .show(ui, add)
        .inner
}

/// Inspector of whatever is selected, on the right.
pub fn right_panel<R>(ui: &mut egui::Ui, theme: &Theme, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Panel::right("right")
        .exact_size(RIGHT_W)
        .resizable(false)
        .frame(egui::Frame::new().fill(theme.panel))
        .show(ui, add)
        .inner
}

/// The cutting mat filling whatever the panels left over, with its caption.
#[allow(
    dead_code,
    reason = "the bare mat a tab starts from, before it paints its own content"
)]
pub fn mat_center(ui: &mut egui::Ui, theme: &Theme, label: &str) {
    egui::CentralPanel::no_frame().show(ui, |ui| {
        let size = ui.available_size();
        let (resp, painter) = widgets::mat_canvas(ui, theme, size);
        widgets::canvas_label(&painter, theme, resp.rect, label);
    });
}
