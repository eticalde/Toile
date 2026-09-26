use eframe::egui::{
    self, CursorIcon, Event, Id, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2,
};
use toile_engine::body::Collider;
use toile_engine::draft::{Doc, PieceKey, PointKey, block};
use toile_engine::session::Session;

use super::super::super::state::{Scope, Selection, State};
use super::super::show;
use crate::config::Paper;
use crate::tabs::patronaje::apply;
use crate::tabs::right_panel;
use crate::theme::Theme;

/// One press or release of the primary button, where the pointer is.
fn press(at: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

/// A product on the table and the right panel over it, with nothing else on
/// the screen: every edit the panel asks for is played the way the tab plays
/// it.
pub(super) struct Desk {
    pub(super) session: Session,
    pub(super) state: State,
    pub(super) ctx: egui::Context,
    /// The pointer the last frame asked the platform for.
    pub(super) cursor: CursorIcon,
    /// The paper the installation prints on, which the panel names and steps.
    pub(super) paper: Paper,
    theme: Theme,
}

impl Desk {
    /// A desk tall enough that the panel never has to scroll: a name that
    /// scrolled out of sight could not be pressed, and would say nothing
    /// about whether it answers a press.
    pub(super) fn tall(doc: Doc) -> Desk {
        let ctx = egui::Context::default();
        let theme = Theme::sastreria();
        theme.apply(&ctx);
        Desk {
            session: Session::from_doc(doc, Collider::demo()).expect("the product opens"),
            state: State::default(),
            ctx,
            cursor: CursorIcon::Default,
            paper: Paper::default(),
            theme,
        }
    }

    /// One frame of the panel fed `events`.
    pub(super) fn frame(&mut self, events: Vec<Event>) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1320.0, 2800.0))),
            events,
            ..RawInput::default()
        };
        let (draft, faults) = (self.session.draft(), self.session.seam_faults());
        let (theme, paper) = (&self.theme, self.paper);
        let state = &mut self.state;
        let piece = state.active;
        let mut asked = Vec::new();
        let pass = self.ctx.run_ui(input, |ui| {
            asked = right_panel(ui, theme, |ui| {
                show(ui, theme, draft, faults, piece, state, paper)
            });
        });
        self.cursor = pass.platform_output.cursor_icon;
        pass.drop_without_applying_deltas();
        apply(&mut self.session, asked, &mut self.state.refused);
    }

    /// The pointer pressing at `from`, dragged to `to` over `steps` frames,
    /// and let go there.
    ///
    /// A frame each, because a rail answers where the pointer is now: a drag
    /// that arrived in one frame would write one value and prove nothing about
    /// the frames in between landing in a single entry.
    pub(super) fn drag(&mut self, from: Pos2, to: Pos2, steps: u8) {
        self.frame(vec![Event::PointerMoved(from)]);
        self.frame(vec![press(from, true)]);
        for step in 1..=steps {
            let along = f32::from(step) / f32::from(steps);
            let at = from + (to - from) * along;
            self.frame(vec![Event::PointerMoved(at)]);
        }
        self.frame(vec![press(to, false)]);
    }

    /// The pointer arriving, pressing and letting go, a frame each.
    pub(super) fn click(&mut self, at: Pos2) {
        self.frame(vec![Event::PointerMoved(at)]);
        for pressed in [true, false] {
            self.frame(vec![press(at, pressed)]);
        }
    }

    /// One key pressed with `modifiers` held.
    pub(super) fn key(&mut self, key: Key, modifiers: Modifiers) {
        self.frame(vec![Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }]);
    }

    /// Text typed at the keyboard.
    pub(super) fn text(&mut self, text: &str) {
        self.frame(vec![Event::Text(text.to_owned())]);
    }

    /// The middle of what was drawn under `id`.
    pub(super) fn centre(&self, id: Id) -> Pos2 {
        self.rect(id).center()
    }

    /// What was drawn under `id`, where the last frame put it.
    pub(super) fn rect(&self, id: Id) -> Rect {
        self.ctx.read_response(id).expect("the panel drew it").rect
    }

    /// Whether the panel drew anything under `id` at all.
    pub(super) fn drew(&self, id: Id) -> bool {
        self.ctx.read_response(id).is_some()
    }

    /// How many entries the product's history would take back.
    pub(super) fn entries(&self) -> usize {
        self.session
            .draft()
            .expect("a product is open")
            .undo_depth()
    }
}

/// The shipped trouser front on a tall desk, open on its hip node with nothing
/// typed yet.
pub(super) fn hip() -> (Desk, PointKey) {
    let doc = block::trouser_front();
    let piece: PieceKey = doc
        .piece_named(block::FRONT)
        .expect("the block draws one piece");
    let point = doc
        .shows_label(piece, "cadera_lat")
        .expect("the block names the hip");
    let mut desk = Desk::tall(doc);
    desk.state.scope = Scope::Piece;
    desk.state.active = Some(piece);
    desk.state.selection = Selection::point(point);
    desk.frame(Vec::new());
    (desk, point)
}

/// The same block, open on the tract that leaves its hip: the one selection
/// the elastic section is drawn for.
pub(super) fn hem() -> (Desk, PointKey) {
    let (mut desk, point) = hip();
    desk.state.selection = Selection::Edge(point);
    desk.frame(Vec::new());
    (desk, point)
}

/// Any product on a tall desk, open on the first piece it holds with nothing
/// chosen: the state a piece's own detail opens in.
pub(super) fn piece_open(doc: Doc) -> (Desk, PieceKey) {
    let piece = doc.piece_keys()[0];
    let mut desk = Desk::tall(doc);
    desk.state.scope = Scope::Piece;
    desk.state.active = Some(piece);
    desk.frame(Vec::new());
    (desk, piece)
}

/// What the product's file says right now.
pub(super) fn bytes(desk: &Desk) -> String {
    desk.session
        .draft()
        .expect("a product is open")
        .doc()
        .to_canonical_json()
}
