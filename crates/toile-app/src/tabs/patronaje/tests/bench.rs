use eframe::egui::{self, Event, Key, Modifiers, Pos2, RawInput, Rect, vec2};
use toile_engine::body::Collider;
use toile_engine::draft::{Doc, MeasureSet, Piece, PieceKey, Point, Winding, block};
use toile_engine::session::Session;

use super::super::{State, active_piece, apply, canvas, follow, layout};
use super::button;
use crate::theme::Theme;

/// A product on the table, what the tab remembers of it, and the context its
/// frames run in: the mat, with nothing around it.
pub(super) struct Bench {
    pub(super) session: Session,
    pub(super) state: State,
    ctx: egui::Context,
    theme: Theme,
}

impl Bench {
    pub(super) fn new(doc: Doc) -> Bench {
        let ctx = egui::Context::default();
        let theme = Theme::sastreria();
        theme.apply(&ctx);
        Bench {
            session: Session::from_doc(doc, Collider::demo()).expect("the product opens"),
            state: State::default(),
            ctx,
            theme,
        }
    }

    /// One frame of the mat fed `events`, played into the session the way the
    /// tab plays it.
    pub(super) fn frame(&mut self, events: Vec<Event>) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1320.0, 780.0))),
            events,
            ..RawInput::default()
        };
        let draft = self.session.draft();
        let piece = active_piece(draft, self.state.active, self.session.piece());
        self.state.active = piece;
        let before = draft
            .map(|held| held.doc().piece_keys())
            .unwrap_or_default();
        let (theme, state) = (&self.theme, &mut self.state);
        let mut verbs = Vec::new();
        let pass = self.ctx.run_ui(input, |ui| {
            verbs = canvas::show(ui, theme, draft, piece, state);
        });
        pass.drop_without_applying_deltas();
        apply(&mut self.session, verbs, &mut self.state.refused);
        follow(&self.session, &mut self.state, &before);
    }

    /// The pointer arriving, pressing and letting go, a frame each.
    pub(super) fn click(&mut self, at: Pos2) {
        self.frame(vec![Event::PointerMoved(at)]);
        self.frame(vec![button(at, true)]);
        self.frame(vec![button(at, false)]);
    }

    /// One key pressed, with no modifier held.
    pub(super) fn key(&mut self, key: Key) {
        self.frame(vec![Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }]);
    }

    pub(super) fn doc(&self) -> &Doc {
        self.session.draft().expect("a product is open").doc()
    }

    /// Where the middle of a piece lies on the glass, as the whole product
    /// draws it right now.
    pub(super) fn on_product(&self, piece: PieceKey) -> Pos2 {
        let draft = self.session.draft().expect("a product is open");
        let laid = layout::of(draft);
        let it = laid.iter().find(|it| it.piece == piece).expect("laid out");
        let bbox = layout::bounds(std::slice::from_ref(it)).expect("an outline");
        let middle = [f64::from(bbox.center().x), f64::from(bbox.center().y)];
        let found = layout::under(middle, &laid, 0.0).map(|it| it.piece);
        assert_eq!(found, Some(piece), "the middle of its box is on the piece");
        self.state.view.to_screen(middle)
    }
}

/// The front and the back of the shipped trousers.
pub(super) fn front_and_back(doc: &Doc) -> (PieceKey, PieceKey) {
    let front = doc
        .piece_named(block::FRONT)
        .expect("the block draws a front");
    let back = doc
        .piece_named(block::BACK)
        .expect("the block draws a back");
    (front, back)
}

/// Two ten centimetre squares, nothing sewn between them.
pub(super) fn two_squares() -> Doc {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
    for x in [0.0, 30.0] {
        let corners = [[x, 0.0], [x + 10.0, 0.0], [x + 10.0, 10.0], [x, 10.0]];
        let points: Vec<_> = corners
            .into_iter()
            .map(|[x, y]| doc.points.insert(Point::at(x, y)))
            .collect();
        doc.pieces
            .insert(Piece::polygon("Cuadro", points, Winding::Cw));
    }
    doc
}
