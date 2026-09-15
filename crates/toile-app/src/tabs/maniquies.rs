mod identity;
mod measures;
mod stand;

use eframe::egui;
use eframe::egui_wgpu::RenderState;
use toile_engine::body;
use toile_engine::draft::Station;
use toile_engine::session::Session;

use self::stand::Stand;
use crate::tabs::{Body, UNNAMED, Workspace, left_panel, right_panel};
use crate::theme::Theme;
use crate::viewport::BodyView;

/// The mannequins tab: the Anny body, its phenotype controls, and the
/// measurements it is solved against.
///
/// With a product on the table the body is the one its pattern resolves
/// against, read from the document and written back through the session, so
/// what is shaped here is what Patronaje draws with. With none, the tab shapes
/// a body held in memory, and says that nothing keeps it.
pub struct State {
    rs: RenderState,
    view: BodyView,
    /// The station mask the body was last coloured with, so a frame that
    /// changes nothing uploads nothing.
    lit: u32,
    stand: Stand,
}

impl State {
    /// Forgets what belonged to the product that was open, called when another
    /// one takes the table.
    pub fn reset(&mut self) {
        self.stand.forget();
    }

    /// The tab with no mesh yet: the first frame it is shown solves one.
    pub fn new(rs: RenderState, theme: &Theme) -> Self {
        let view = BodyView::new(&rs, theme);
        Self {
            rs,
            view,
            lit: 0,
            stand: Stand::default(),
        }
    }

    /// The body on the stand, as the status bar has to name it.
    ///
    /// Read where the measures panel reads it, so the bar cannot come to
    /// disagree with the panel above it.
    pub fn body<'a>(&'a self, session: &'a Session) -> Body<'a> {
        let set = self.stand.body(session);
        Body {
            name: if set.name.is_empty() {
                UNNAMED
            } else {
                &set.name
            },
            measures: set.values.len(),
            kept: Stand::kept(session),
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

/// Whether a pointer is still down on a control, which is what keeps the
/// gesture it opened from closing.
fn pressed(response: &egui::Response) -> bool {
    response.is_pointer_button_down_on() || response.dragged()
}

pub fn show(ui: &mut egui::Ui, w: &mut Workspace<'_>) {
    let theme = w.theme;
    let session = &mut *w.session;
    let st = &mut *w.maniquies;
    // The history is the product's, not the mat's, so its keys work from here
    // too. A text box in hand keeps them: there Cmd+Z undoes the typing.
    let step = ui.input(|i| {
        (i.modifiers.command && i.key_pressed(egui::Key::Z)).then_some(i.modifiers.shift)
    });
    if let Some(redo) = step
        && !ui.ctx().egui_wants_keyboard_input()
    {
        st.stand.step(session, redo);
    }
    left_panel(ui, theme, |ui| {
        identity::panel(ui, theme, session, &mut st.stand);
    });
    right_panel(ui, theme, |ui| {
        measures::panel(ui, theme, session, &mut st.stand);
    });
    // Closed before the body is compared, so a slider let go of this frame
    // rebuilds on this frame.
    if st.stand.settle(session) {
        ui.ctx().request_repaint();
    }
    egui::CentralPanel::no_frame().show(ui, |ui| {
        let size = ui.available_size();
        let mask = st
            .stand
            .highlight
            .as_deref()
            .map_or(0, |name| mask_of(body::stations_for(name)));
        if st.stand.due(session) {
            let mesh = st.stand.rebuild(session);
            st.view.set_mesh(&st.rs, &mesh, mask);
            st.lit = mask;
            // The panels were drawn from the solve before this one.
            ui.ctx().request_repaint();
        } else if mask != st.lit {
            st.view.set_highlight(&st.rs, mask);
            st.lit = mask;
        }
        st.view.show(ui, size, &st.rs, theme);
    });
    identity::dialog(ui.ctx(), theme, session, &mut st.stand);
}

#[cfg(test)]
mod tests;
