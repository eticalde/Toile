pub mod maniquies;
pub mod patronaje;
pub mod probador;
pub mod telas;

use eframe::egui;
use toile_engine::session::Session;

use crate::theme::Theme;

/// Left panel width, in points, from the layout mockups.
const LEFT_W: f32 = 232.0;
const RIGHT_W: f32 = 296.0;

/// What a panel calls a body nobody has named yet.
///
/// A document made from scratch carries one, and its name is the empty string:
/// drawn as it stands it leaves a box that reads as a box which failed to fill,
/// and nobody can tell that from a name the document never held.
pub const UNNAMED: &str = "sin nombre";

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

    /// The cells of the status bar, left to right, each saying whether it is
    /// an alert.
    ///
    /// Two tab states come along because some of what the bar has to report is
    /// not in the document: an edit the session refused leaves the drawing
    /// untouched, and the body being measured belongs to the mannequin tab
    /// alone. The bar is the only place that says either.
    ///
    /// Telas answers with nothing, and that is the honest answer: the tab holds
    /// no fabric the app can act on, so it has nothing to report.
    pub fn status(
        self,
        session: &Session,
        patronaje: &patronaje::State,
        body: Body<'_>,
    ) -> Vec<(String, bool)> {
        let cells = match self {
            Self::Maniquies => vec![
                body.name.to_owned(),
                format!("{} medidas", body.measures),
                "cm".to_owned(),
            ],
            Self::Patronaje => return patronaje::status(session, patronaje),
            Self::Telas => Vec::new(),
            Self::Probador => fitting(session),
        };
        cells.into_iter().map(|cell| (cell, false)).collect()
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

/// What the fitting table can report about a drape.
///
/// The empty snapshot a table with no sim thread answers with has `converged`
/// false, exactly as a simulation still working does, so the sim cells are
/// gated on there being a thread at all rather than on what the snapshot says.
fn fitting(session: &Session) -> Vec<String> {
    if !session.simulating() {
        return vec!["sin simulación".to_owned()];
    }
    let snap = session.snapshot();
    let sim = if snap.converged {
        "sim dormida (0% CPU)"
    } else {
        "sim corriendo"
    };
    vec![
        format!("substeps {}", snap.substeps),
        sim.to_owned(),
        format!("derive {:.1} ms", session.last_derive_ms),
    ]
}

/// The body the mannequin tab holds, as the status bar has to name it.
///
/// The two fields the bar reads, rather than the tab's whole state: the tab
/// owns a GPU view and cannot be built without one, and a bar that can be
/// handed a body is a bar whose cells can be checked.
#[derive(Debug, Clone, Copy)]
pub struct Body<'a> {
    /// The name the tab's own measures panel shows in its header.
    pub name: &'a str,
    /// How many measurements that panel lists.
    pub measures: usize,
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
    pub maniquies: &'a mut maniquies::State,
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

#[cfg(test)]
mod tests;
