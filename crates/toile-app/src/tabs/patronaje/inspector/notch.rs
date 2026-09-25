use eframe::egui::{self, Id};
use toile_engine::draft::{
    Command, Doc, EdgeAnchor, Identity, Notch, NotchCount, NotchKey, PieceKey, PointKey,
};

use super::super::slide::SLIDE;
use super::super::state::{Field, State};
use super::super::wire::Verb;
use super::cite::Cite;
use super::elastic::press;
use super::write::{self, Asked};
use crate::theme::Theme;
use crate::widgets::{Editable, field_row, footer_note, formula_row, plain_note, section_with};

const NONE: &str = "Un piquete es un corte en el margen que dice dónde encaja este tramo con el \
                    que se le cose. Sin él, dos lados de largo parecido se montan torcidos y no \
                    hay nada en el papel que lo avise.";
const HELD: &str = "El piquete se corta a mitad del tramo y se dibuja cruzándolo: de qué lado \
                    queda el margen no lo dice el contorno. Se arrastra con el ratón a lo largo \
                    de su tramo, y no sale de él.";
const NODE: &str = "Un piquete sobre un nodo queda debajo del punto y el ratón no puede \
                    distinguirlos: con el tramo elegido —como ahora— la mesa entrega el piquete, \
                    y aquí se escribe a mano el porcentaje.";
const ALONG: &str = "% del tramo, desde su nodo · 0 a 100";
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
    writing: (&mut State, &Cite),
    verbs: &mut Vec<Verb>,
) -> Option<Asked> {
    let cut: Vec<(NotchKey, Notch)> = on(doc, at);
    section_with(ui, theme, "Piquete", &cut.len().to_string());
    let Some(&(key, held)) = cut.first() else {
        offer(ui, theme, at, verbs);
        return None;
    };
    marked(ui, theme, (key, held), cut.len(), writing, verbs)
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
    writing: (&mut State, &Cite),
    verbs: &mut Vec<Verb>,
) -> Option<Asked> {
    let asked = along(ui, theme, (key, held), writing);
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
    if on_a_node(held.at.t) {
        plain_note(ui, theme, NODE);
    }
    footer_note(ui, theme, HELD);
    asked
}

/// Where the mark sits along its tract, as a percentage that can be typed.
///
/// Typing is the whole of what this row adds, and a mark cut at a node needs
/// it: there the mat draws the cut under the node's own dot, so the pointer
/// reaches the mark only with its tract already chosen, and somebody who would
/// rather not aim at all writes the number here.
fn along(
    ui: &mut egui::Ui,
    theme: &Theme,
    (key, held): (NotchKey, Notch),
    (state, cite): (&mut State, &Cite),
) -> Option<Asked> {
    let source = percent(held.at.t);
    let shown = Editable {
        label: "a lo largo",
        source: &source,
        note: ALONG,
        fault: false,
        held: None,
    };
    let of = Field::Along(held.at.from);
    let written = write::field(ui, state, cite, of, &shown, |ui, id, row| {
        formula_row(ui, theme, id, row)
    })?;
    // Nothing is written until the number really changes. The box comes up with
    // the mark's own fraction already in it, and a click in and out of a box
    // confirms what it holds, so without this an entry of the history would be
    // opened for a fraction nobody touched.
    let written = written.trim();
    if written == source {
        return None;
    }
    let t = written.parse::<f64>().ok()? / 100.0;
    let to = EdgeAnchor { t, ..held.at };
    Some((SLIDE, Command::MoveNotch { notch: key, to }))
}

/// Whether the mark sits on one of its tract's two nodes, which is where the
/// pointer cannot tell the cut from the node's own dot.
fn on_a_node(t: f64) -> bool {
    t <= f64::EPSILON || t >= 1.0 - f64::EPSILON
}

/// A fraction as the percentage the row shows it as.
///
/// Two decimals: that is the resolution a drag writes a fraction at, so the
/// number in the box is the number the file holds and confirming it writes no
/// rounding back. The zeros are trimmed because a mark nobody has moved along
/// its tract reads as a whole number.
fn percent(t: f64) -> String {
    let text = format!("{:.2}", t * 100.0);
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The row shows the fraction the file holds, at the resolution a drag
    /// writes one at, with no trailing zeros to type past.
    #[test]
    fn a_fraction_reads_as_the_percentage_of_its_tract() {
        assert_eq!(percent(0.0), "0");
        assert_eq!(percent(0.5), "50");
        assert_eq!(percent(1.0), "100");
        assert_eq!(percent(0.1379), "13.79");
        assert_eq!(percent(0.062), "6.2");
        // A fraction finer than the row can show rounds in the box and not in
        // the document: what the box holds is what it showed, so nothing is
        // written unless somebody types another number.
        assert_eq!(percent(1.0 / 3.0), "33.33");
    }

    /// Both ends of a tract are places where the cut cannot be told from the
    /// node under it.
    #[test]
    fn a_mark_at_either_end_of_its_tract_sits_on_a_node() {
        assert!(on_a_node(0.0), "the node the tract leaves");
        assert!(on_a_node(1.0), "and the one it reaches");
        assert!(!on_a_node(0.5) && !on_a_node(0.01));
    }
}
