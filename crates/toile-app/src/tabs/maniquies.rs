mod identity;
mod measures;
mod people;
mod stand;

use eframe::egui;
use eframe::egui_wgpu::RenderState;
use toile_engine::draft::{BodyMesh, MeasureSet};
use toile_engine::session::Session;

use self::people::People;
use self::stand::Stand;
use crate::fitting::Hand;
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
    /// The body last solved, kept so another row's tape can be laid on it
    /// without solving it again.
    mesh: Option<BodyMesh>,
    stand: Stand,
    people: People,
}

impl State {
    /// Forgets what belonged to the product that was open, called when another
    /// one takes the table.
    pub fn reset(&mut self) {
        self.stand.forget();
        self.people.forget();
    }

    /// The tab with no mesh yet: the first frame it is shown solves one.
    pub fn new(rs: RenderState, theme: &Theme) -> Self {
        let view = BodyView::new(&rs, theme);
        Self {
            rs,
            view,
            mesh: None,
            stand: Stand::default(),
            people: People::default(),
        }
    }

    /// The body shaped here while no product is open.
    ///
    /// Handed out rather than kept to itself: with nothing on the table this
    /// is the only body there is, and the cloth has to fall on the very one
    /// these sliders write into.
    pub fn loose(&self) -> &MeasureSet {
        self.stand.loose()
    }

    /// Whether a control in this tab is in hand.
    ///
    /// The program asks before it re-solves the body the cloth falls on: a
    /// value a hand is still moving is not one worth baking, and this tab
    /// already holds its own view back on the same rule.
    pub fn hand(&self) -> Hand {
        if self.stand.held() {
            Hand::Holding
        } else {
            Hand::Free
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
    let plea = left_panel(ui, theme, |ui| {
        identity::panel(ui, theme, session, &mut st.stand);
        people::panel(ui, theme, w.shelf, &mut st.people, Stand::kept(session))
    });
    if let Some(plea) = plea {
        people::act(
            plea,
            session,
            &mut st.stand,
            w.shelf,
            w.band,
            &mut st.people,
        );
        ui.ctx().request_repaint();
    }
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
        if st.stand.due(session) {
            let mesh = st.stand.rebuild(session);
            st.view.set_mesh(&st.rs, &mesh);
            st.mesh = Some(mesh);
            // The panels were drawn from the solve before this one.
            ui.ctx().request_repaint();
        }
        if let Some(mesh) = &st.mesh
            && st.stand.tape_due()
        {
            let tape = st.stand.lay(mesh);
            st.view.set_tape(&st.rs, tape.as_ref());
        }
        st.view.show(ui, size, &st.rs, theme);
    });
    identity::dialog(ui.ctx(), theme, session, &mut st.stand);
}

#[cfg(test)]
mod tests;
