use eframe::egui::{self, Id};
use toile_engine::couture::HOLDS_ITS_RATIO;
use toile_engine::draft::{
    Command, Doc, EdgeRange, Elastic, ElasticKey, Identity, PieceKey, PointKey,
};

use super::super::state::State;
use super::super::wire::Verb;
use crate::theme::Theme;
use crate::widgets::{PAD, Track, button_named, footer_note, section, track};

const NONE: &str = "Un elástico sujeta el tramo a una fracción del largo con que se dibujó, y \
                    tira hasta que el cuerpo lo detiene. Sin él solo el roce sostiene la prenda.";
const HELD: &str = "100 % es el largo dibujado. La rigidez es cuánto resiste la banda a que \
                    la estiren: 10 la mantiene cerca de su razón, 0,1 la deja ceder el doble. \
                    Por encima de 3 apenas cambia.";

const PUT_ON: &str = "poner un elástico";
const TAKE_OFF: &str = "quitar el elástico";
const RATIO: &str = "ajustar la razón del elástico";
const STRENGTH: &str = "ajustar la rigidez del elástico";

/// What the ratio rail offers, as a percentage of the drawn length.
///
/// A hundred is the tract exactly as it was drawn and two hundred is the
/// ceiling the document itself keeps. The low end stops well short of the zero
/// no elastic carries, so no drag can ask for a stretch pulled to no length at
/// all and be refused for it.
const RATIO_SPAN: (f64, f64) = (20.0, 200.0);

/// What the strength rail offers, as a multiple of a band of strength one.
///
/// Measured on the seeded skirt three simulated seconds in, the waistband
/// stands at 120 % of the length it is held to at the default of ten and 143 %
/// at the slack end, and the firm band sits 10.7 cm higher for it. The rail
/// does its work down there: the knee is near three, where the band already
/// stands at 122 %, so everything from there to fifty is one firm band told
/// apart by a couple of points. `tests/seeding/grip.rs` reads all three.
const STRENGTH_SPAN: (f64, f64) = (0.1, 50.0);

// A rail that reached under the document's floor would offer a drag the edit
// refuses, so the two are held together where the build can see them.
const _: () = assert!(STRENGTH_SPAN.0 >= Elastic::MIN_STRENGTH);

/// The rail a drag has hold of, and whether a pointer is still on it.
///
/// A matter of view like every other field of the tab's state: what is open is
/// an undo entry of the product's own history, and this only remembers which
/// rail opened it so that the next frame of the same drag writes into it
/// instead of starting another.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Grip {
    held: Option<Pull>,
    holding: bool,
}

/// Which of an elastic's two numbers a rail writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pull {
    Ratio(ElasticKey),
    Strength(ElasticKey),
}

/// What holds the chosen tract in, and the ways to change it.
///
/// Nothing is written by looking: a tract with no elastic offers one and
/// writes nothing until the press, and a tract with one draws what the
/// document holds. Opening a product and reading this section leaves its bytes
/// and its version exactly as they were.
pub fn show(
    ui: &mut egui::Ui,
    theme: &Theme,
    doc: &Doc,
    at: (PieceKey, PointKey, PointKey),
    (state, verbs): (&mut State, &mut Vec<Verb>),
) {
    let (piece, from, to) = at;
    let stretch = EdgeRange::between(piece, from, to);
    section(ui, theme, "Elástico");
    match on(doc, stretch) {
        Some((key, held)) => rails(ui, theme, (key, held), (state, verbs)),
        None => offer(ui, theme, (from, stretch), verbs),
    }
}

/// Closes the entry of a rail nobody is holding any more.
///
/// Called wherever the panel ends, and not from the section, so a drag that
/// ends by the selection changing under it closes its entry too.
pub fn settle(state: &mut State) -> Option<Verb> {
    if std::mem::take(&mut state.grip.holding) {
        return None;
    }
    state.grip.held.take().map(|_| Verb::End)
}

/// The elastic on one stretch of contour, when the document holds one.
///
/// Node to node and nothing else. The panel writes exactly that stretch, so a
/// row it draws is a row it can write back; an elastic anchored inside a tract
/// came from somewhere else and is left to whatever made it. Two elastics over
/// one tract are allowed, as two seams are, and the first the arena holds is
/// the one these rails edit.
fn on(doc: &Doc, at: EdgeRange) -> Option<(ElasticKey, Elastic)> {
    doc.elastics
        .iter()
        .find(|(_, held)| covers(held.at, at))
        .map(|(key, held)| (key, *held))
}

/// Whether a stretch runs from node to node over exactly the tract `at`.
fn covers(held: EdgeRange, at: EdgeRange) -> bool {
    let ends =
        held.head.t.to_bits() == 0.0_f64.to_bits() && held.tail.t.to_bits() == 0.0_f64.to_bits();
    ends && held.head.piece == at.head.piece
        && held.head.from == at.head.from
        && held.tail.from == at.tail.from
}

/// What a tract with no elastic shows: what one would do, and the press that
/// puts it on.
///
/// It goes on at the very length the tract was drawn at, so the press says
/// "this stretch is elastic" and leaves the drape where it stood until someone
/// pulls it in. An elastic that arrived already tightened would move the cloth
/// on a press that was about the tract, not about the fit.
fn offer(ui: &mut egui::Ui, theme: &Theme, at: (PointKey, EdgeRange), verbs: &mut Vec<Verb>) {
    let pressed = press(ui, theme, put_on_id(at.0), "Poner elástico");
    footer_note(ui, theme, NONE);
    if pressed {
        let elastic = Elastic::new(at.1, Elastic::NEUTRAL_RATIO, HOLDS_ITS_RATIO);
        verbs.extend(super::super::entry(
            PUT_ON,
            Command::AddElastic {
                identity: Identity::New,
                elastic,
            },
        ));
    }
}

/// The two numbers of an elastic already on the tract, and the way to take it
/// off.
fn rails(
    ui: &mut egui::Ui,
    theme: &Theme,
    (key, held): (ElasticKey, Elastic),
    (state, verbs): (&mut State, &mut Vec<Verb>),
) {
    let ratio = Track {
        label: "razón",
        span: RATIO_SPAN,
        step: 1.0,
        value: held.ratio * 100.0,
        unit: "%",
        decimals: 0,
    };
    let asked = rail(ui, theme, &ratio, (ratio_id(key), state));
    if let Some(to) = asked
        .map(|percent| percent / 100.0)
        .filter(moves(held.ratio))
    {
        grip(state, verbs, Pull::Ratio(key), RATIO);
        verbs.push(Verb::Edit(Box::new(Command::SetElasticRatio {
            elastic: key,
            to,
        })));
    }
    let firm = Track {
        label: "rigidez",
        span: STRENGTH_SPAN,
        step: 0.1,
        value: held.strength,
        unit: "×",
        decimals: 1,
    };
    let asked = rail(ui, theme, &firm, (strength_id(key), state));
    if let Some(to) = asked.filter(moves(held.strength)) {
        grip(state, verbs, Pull::Strength(key), STRENGTH);
        verbs.push(Verb::Edit(Box::new(Command::SetElasticStrength {
            elastic: key,
            to,
        })));
    }
    if press(ui, theme, take_off_id(key), "Quitar elástico") {
        verbs.extend(super::super::entry(
            TAKE_OFF,
            Command::RemoveElastic { elastic: key },
        ));
    }
    footer_note(ui, theme, HELD);
}

/// One press of a section of the panel, set in from its edge like every other
/// control on it, under an identity the caller names.
pub(super) fn press(ui: &mut egui::Ui, theme: &Theme, id: Id, label: &str) -> bool {
    ui.add_space(6.0);
    let pressed = ui
        .horizontal(|ui| {
            ui.add_space(PAD);
            button_named(ui, theme, id, label).clicked()
        })
        .inner;
    ui.add_space(4.0);
    pressed
}

/// Whether a rail is asking for a number the document does not already hold.
///
/// Compared as bits, because that is the question: a press that lands on the
/// value already stored has moved nothing, and opening an undo entry for it
/// would leave a step in the history that undoes to itself.
fn moves(stored: f64) -> impl Fn(&f64) -> bool {
    move |&asked| asked.to_bits() != stored.to_bits()
}

/// One rail drawn, kept in hand while the pointer stays on it.
///
/// A pointer down on it says so before anything is written, because the frame
/// that opens an entry is a frame the hand is already holding: read the other
/// way round the entry would be closed by the settle at the end of the very
/// frame that opened it, and a drag would leave one entry per frame.
fn rail(
    ui: &mut egui::Ui,
    theme: &Theme,
    row: &Track<'_>,
    (id, state): (Id, &mut State),
) -> Option<f64> {
    let (resp, asked) = track(ui, theme, id, row);
    if resp.is_pointer_button_down_on() || resp.dragged() {
        state.grip.holding = true;
    }
    asked
}

/// Takes a rail in hand, opening its entry unless it is the one already open.
///
/// Every frame of the drag folds into that one entry: letting go is the
/// deliberate act, and no frame before it is. Another rail taken while one is
/// open closes the first, so the two numbers never share a step.
fn grip(state: &mut State, verbs: &mut Vec<Verb>, want: Pull, label: &'static str) {
    if state.grip.held == Some(want) {
        return;
    }
    if state.grip.held.is_some() {
        verbs.push(Verb::End);
    }
    verbs.push(Verb::Begin(label));
    state.grip.held = Some(want);
}

/// The identity egui keeps the ratio rail under.
///
/// Named for the elastic and never for the row's place on the panel, so the
/// hand stays on the same rail however the rows above it move.
pub(in crate::tabs::patronaje) fn ratio_id(at: ElasticKey) -> Id {
    Id::new(("patronaje-elastic-ratio", at))
}

/// The identity of the strength rail.
pub(in crate::tabs::patronaje) fn strength_id(at: ElasticKey) -> Id {
    Id::new(("patronaje-elastic-strength", at))
}

/// The identity of the press that takes an elastic off.
pub(in crate::tabs::patronaje) fn take_off_id(at: ElasticKey) -> Id {
    Id::new(("patronaje-elastic-off", at))
}

/// The identity of the press that puts one on the tract leaving `node`.
pub(in crate::tabs::patronaje) fn put_on_id(node: PointKey) -> Id {
    Id::new(("patronaje-elastic-on", node))
}
