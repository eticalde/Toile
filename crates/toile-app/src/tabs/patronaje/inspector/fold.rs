use eframe::egui::{self, Id};
use toile_engine::draft::{
    Command, Defect, Draft, EdgeRange, Identity, PieceKey, PointKey, Symmetry, SymmetryKey,
};

use super::super::wire::Verb;
use super::elastic::press;
use crate::theme::Theme;
use crate::widgets::{field_row, footer_note, plain_note, section_with};

const NONE: &str = "Media pieza al doblez: la tela es lo dibujado más su espejo, unidos por este \
                    borde y sin costura. Así se dibuja la mitad de los patrones del oficio — una \
                    pretina, una vista, un canesú — y Toile corta, malla, drapea, mide y exporta \
                    la pieza entera.";
const HELD: &str = "El eje se dibuja a trazos gruesos sobre la mesa, y la mitad espejada en tinta \
                    tenue: está dibujada, no se edita. Lo que se traza en la mitad dibujada sale \
                    en las dos al exportar.";
const ELSEWHERE: &str = "Esta pieza ya va al doblez por otro borde. Quítalo primero: un segundo \
                         eje sería un cuarto de pieza, y eso no es esto.";
const BROKEN: &str = "Con este eje la pieza se dobla sobre sí misma. El contorno tiene que quedar \
                      entero de un solo lado del doblez.";

const PUT_ON: &str = "poner el doblez";
const TAKE_OFF: &str = "quitar el doblez";

/// The fold the chosen tract could be, or the one it already is.
///
/// Nothing is written by looking: a tract with no axis offers one and writes
/// nothing until the press. Opening a product and reading this section leaves
/// its bytes and its version exactly as they were.
pub fn show(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: &Draft,
    at: (PieceKey, PointKey, PointKey),
    verbs: &mut Vec<Verb>,
) {
    let (piece, from, to) = at;
    let held = draft.doc().symmetry_of(piece);
    let on_this_tract = held.filter(|(_, axis)| runs_along(axis.axis, at));
    section_with(
        ui,
        theme,
        "Doblez",
        if held.is_some() { "sí" } else { "no" },
    );
    match (on_this_tract, held) {
        (Some((key, _)), _) => marked(ui, theme, key, verbs),
        (None, Some(_)) => plain_note(ui, theme, ELSEWHERE),
        (None, None) => offer(ui, theme, (piece, from, to), verbs),
    }
    // An axis the piece cannot be mirrored across leaves it with no cloth to
    // cut and a defect to say so; the panel says what to do about it.
    let broken = draft
        .defects(piece)
        .iter()
        .any(|defect| matches!(defect, Defect::FoldAxis | Defect::Cloth(_)));
    if broken {
        plain_note(ui, theme, BROKEN);
    }
}

/// Whether an axis is the very tract that is chosen, node to node.
fn runs_along(axis: EdgeRange, at: (PieceKey, PointKey, PointKey)) -> bool {
    let (piece, from, to) = at;
    axis == EdgeRange::between(piece, from, to)
}

/// What a tract that is not the axis shows: what a fold is, and the press that
/// makes this edge one.
fn offer(
    ui: &mut egui::Ui,
    theme: &Theme,
    (piece, from, to): (PieceKey, PointKey, PointKey),
    verbs: &mut Vec<Verb>,
) {
    let pressed = press(ui, theme, put_on_id(from), "Doblar por aquí");
    footer_note(ui, theme, NONE);
    if pressed {
        verbs.extend(super::super::entry(
            PUT_ON,
            Command::AddSymmetry {
                identity: Identity::New,
                symmetry: Symmetry::fold(EdgeRange::between(piece, from, to)),
            },
        ));
    }
}

/// The axis this tract already is, and the press that unmakes it.
fn marked(ui: &mut egui::Ui, theme: &Theme, key: SymmetryKey, verbs: &mut Vec<Verb>) {
    field_row(ui, theme, "eje", "este borde", "");
    if press(ui, theme, take_off_id(key), "Quitar doblez") {
        verbs.extend(super::super::entry(
            TAKE_OFF,
            Command::RemoveSymmetry { symmetry: key },
        ));
    }
    footer_note(ui, theme, HELD);
}

/// The identity of the press that folds a piece on the tract leaving `node`.
pub(in crate::tabs::patronaje) fn put_on_id(node: PointKey) -> Id {
    Id::new(("patronaje-fold-on", node))
}

/// The identity of the press that unfolds it.
pub(in crate::tabs::patronaje) fn take_off_id(at: SymmetryKey) -> Id {
    Id::new(("patronaje-fold-off", at))
}
