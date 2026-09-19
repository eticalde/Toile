use eframe::egui::epaint::{ClippedShape, Shape};
use eframe::egui::{self, Event, Id, Key, Modifiers, Pos2, RawInput, Rect, pos2, vec2};
use toile_engine::body::Collider;
use toile_engine::draft::{Doc, PointKey};
use toile_engine::session::Session;

use super::super::{State, table};
use super::button;
use crate::theme::Theme;

/// The row of the whole product, in the tree.
pub(super) const WHOLE_ROW: Pos2 = pos2(100.0, 50.0);
/// The trousers' front, the first piece row under it.
pub(super) const FRONT_ROW: Pos2 = pos2(100.0, 79.0);
/// The trousers' back, the row after the front.
pub(super) const BACK_ROW: Pos2 = pos2(100.0, 108.0);
/// The row that starts a piece, under the trousers' two.
pub(super) const PLUS_ROW: Pos2 = pos2(100.0, 137.0);
/// The first step of the trail over a piece, where the mat starts past the
/// left panel.
pub(super) const TRAIL: Pos2 = pos2(282.0, 34.0);

/// A product on the table and the whole tab over it: the product tree and the
/// tools on the left, the inspector on the right, the mat between them, drawn
/// and played in the order the application draws and plays them.
pub(super) struct Studio {
    pub(super) session: Session,
    pub(super) state: State,
    pub(super) ctx: egui::Context,
    pub(super) theme: Theme,
    /// What the last frame painted.
    pub(super) shapes: Vec<ClippedShape>,
}

impl Studio {
    pub(super) fn new(doc: Doc) -> Studio {
        let ctx = egui::Context::default();
        let theme = Theme::sastreria();
        theme.apply(&ctx);
        Studio {
            session: Session::from_doc(doc, Collider::demo()).expect("the product opens"),
            state: State::default(),
            ctx,
            theme,
            shapes: Vec::new(),
        }
    }

    /// One frame of the whole tab fed `events`.
    pub(super) fn frame(&mut self, events: Vec<Event>) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1320.0, 780.0))),
            events,
            ..RawInput::default()
        };
        let (theme, session, state) = (&self.theme, &mut self.session, &mut self.state);
        let mut pass = self
            .ctx
            .run_ui(input, |ui| table(ui, theme, session, state));
        self.shapes = std::mem::take(&mut pass.shapes);
        pass.drop_without_applying_deltas();
    }

    /// The pointer arriving, pressing and letting go, a frame each.
    pub(super) fn click(&mut self, at: Pos2) {
        self.frame(vec![Event::PointerMoved(at)]);
        self.frame(vec![button(at, true)]);
        self.frame(vec![button(at, false)]);
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
            .expect("the tab drew it")
            .rect
            .center()
    }

    /// Where a point lies on the glass, as the mat draws it right now.
    pub(super) fn on_glass(&self, point: PointKey) -> Pos2 {
        let draft = self.session.draft().expect("a product is open");
        let cm = draft.resolved(point).expect("the point resolves");
        self.state.view.to_screen(cm)
    }

    pub(super) fn doc(&self) -> &Doc {
        self.session.draft().expect("a product is open").doc()
    }
}

/// Every shape the last frame painted, with the groups opened up.
pub(super) fn painted(studio: &Studio) -> Vec<&Shape> {
    fn open<'a>(shape: &'a Shape, out: &mut Vec<&'a Shape>) {
        match shape {
            Shape::Vec(inner) => inner.iter().for_each(|shape| open(shape, out)),
            other => out.push(other),
        }
    }
    let mut out = Vec::new();
    for clipped in &studio.shapes {
        open(&clipped.shape, &mut out);
    }
    out
}
