use eframe::egui::{self, Id};
use toile_engine::draft::{
    Command, Doc, EdgeAnchor, Identity, Notch, NotchCount, NotchKey, PieceKey, PointKey,
};

use super::super::wire::Verb;
use super::elastic::press;
use crate::theme::Theme;
use crate::widgets::{field_row, footer_note, plain_note, section_with};

const NONE: &str = "Un piquete es un corte en el margen que dice dónde encaja este tramo con el \
                    que se le cose. Sin él, dos lados de largo parecido se montan torcidos y no \
                    hay nada en el papel que lo avise.";
const HELD: &str = "El piquete se corta a mitad del tramo y se dibuja cruzándolo: de qué lado \
                    queda el margen no lo dice el contorno.";
const MANY: &str = "Este tramo lleva más de un piquete. La mesa los dibuja todos; aquí se edita \
                    el primero.";

const PUT_ON: &str = "poner un piquete";
const TAKE_OFF: &str = "quitar el piquete";

/// Where a notch this panel puts on lands along the chosen tract.
///
/// The middle, because that is the one place on a tract nobody has to be told
/// about: at a node it would sit under the node's own dot and could not be told
/// from it, and the tract is what was chosen.
const MIDDLE: f64 = 0.5;

/// The notches cut into the chosen tract, and the presses that change them.
///
/// Nothing is written by looking: a tract with no notch offers one and writes
/// nothing until the press. Opening a product and reading this section leaves
/// its bytes and its version exactly as they were.
pub fn show(
    ui: &mut egui::Ui,
    theme: &Theme,
    doc: &Doc,
    at: (PieceKey, PointKey),
    verbs: &mut Vec<Verb>,
) {
    let cut: Vec<(NotchKey, Notch)> = on(doc, at);
    section_with(ui, theme, "Piquete", &cut.len().to_string());
    match cut.first() {
        Some(&(key, held)) => marked(ui, theme, (key, held), cut.len(), verbs),
        None => offer(ui, theme, at, verbs),
    }
}

/// Every notch cut into one tract, in key order.
///
/// The tract is named by the node it leaves, which is exactly what an anchor
/// names, so this asks the one question the document can answer without
/// measuring anything.
fn on(doc: &Doc, at: (PieceKey, PointKey)) -> Vec<(NotchKey, Notch)> {
    let (piece, from) = at;
    doc.notches
        .iter()
        .filter(|(_, held)| held.at.piece == piece && held.at.from == from)
        .map(|(key, held)| (key, *held))
        .collect()
}

/// What a tract with no notch shows: what one is for, and the press that cuts
/// it.
fn offer(ui: &mut egui::Ui, theme: &Theme, at: (PieceKey, PointKey), verbs: &mut Vec<Verb>) {
    let pressed = press(ui, theme, put_on_id(at.1), "Poner piquete");
    footer_note(ui, theme, NONE);
    if pressed {
        let notch = Notch::lone(EdgeAnchor {
            piece: at.0,
            from: at.1,
            t: MIDDLE,
        });
        verbs.extend(super::super::entry(
            PUT_ON,
            Command::AddNotch {
                identity: Identity::New,
                notch,
                mate: None,
            },
        ));
    }
}

/// The notch already on the tract: where it sits, how many cuts it is, and the
/// press that takes it off.
fn marked(
    ui: &mut egui::Ui,
    theme: &Theme,
    (key, held): (NotchKey, Notch),
    count: usize,
    verbs: &mut Vec<Verb>,
) {
    let along = format!("{:.0}", held.at.t * 100.0);
    field_row(ui, theme, "a lo largo", &along, "%");
    field_row(ui, theme, "cortes", cuts(held.count), "");
    let mate = if held.mate.is_some() { "sí" } else { "no" };
    field_row(ui, theme, "con pareja", mate, "");
    if press(ui, theme, take_off_id(key), "Quitar piquete") {
        verbs.extend(super::super::entry(
            TAKE_OFF,
            Command::RemoveNotch { notch: key },
        ));
    }
    if count > 1 {
        plain_note(ui, theme, MANY);
    }
    footer_note(ui, theme, HELD);
}

/// How many cuts the mark is, in words.
fn cuts(count: NotchCount) -> &'static str {
    match count {
        NotchCount::Single => "uno",
        NotchCount::Double => "dos",
        NotchCount::Triple => "tres",
    }
}

/// The identity of the press that cuts a notch into the tract leaving `node`.
pub(in crate::tabs::patronaje) fn put_on_id(node: PointKey) -> Id {
    Id::new(("patronaje-notch-on", node))
}

/// The identity of the press that takes one off.
pub(in crate::tabs::patronaje) fn take_off_id(at: NotchKey) -> Id {
    Id::new(("patronaje-notch-off", at))
}
