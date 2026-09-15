use eframe::egui::{
    self, CursorIcon, Event, Id, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2,
};
use toile_engine::draft::{Doc, PieceKey, PointKey, block};
use toile_engine::session::Session;

use super::super::super::state::{Scope, Selection, State};
use super::super::show;
use crate::tabs::patronaje::{apply, entry};
use crate::tabs::right_panel;
use crate::theme::Theme;

/// A product on the table and the right panel over it, with nothing else on
/// the screen: every edit the panel asks for is played the way the tab plays
/// it.
pub(super) struct Desk {
    pub(super) session: Session,
    pub(super) state: State,
    pub(super) ctx: egui::Context,
    /// The pointer the last frame asked the platform for.
    pub(super) cursor: CursorIcon,
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
            session: Session::from_doc(doc).expect("the product opens"),
            state: State::default(),
            ctx,
            cursor: CursorIcon::Default,
            theme,
        }
    }

    /// One frame of the panel fed `events`.
    pub(super) fn frame(&mut self, events: Vec<Event>) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1320.0, 2400.0))),
            events,
            ..RawInput::default()
        };
        let draft = self.session.draft();
        let (theme, state) = (&self.theme, &mut self.state);
        let piece = state.active;
        let mut asked = None;
        let pass = self.ctx.run_ui(input, |ui| {
            asked = right_panel(ui, theme, |ui| show(ui, theme, draft, piece, state));
        });
        self.cursor = pass.platform_output.cursor_icon;
        pass.drop_without_applying_deltas();
        if let Some((label, command)) = asked {
            apply(
                &mut self.session,
                entry(label, command),
                &mut self.state.refused,
            );
        }
    }

    /// The pointer arriving, pressing and letting go, a frame each.
    pub(super) fn click(&mut self, at: Pos2) {
        self.frame(vec![Event::PointerMoved(at)]);
        for pressed in [true, false] {
            self.frame(vec![Event::PointerButton {
                pos: at,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            }]);
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
        self.ctx
            .read_response(id)
            .expect("the panel drew it")
            .rect
            .center()
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
