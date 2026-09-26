mod cite;
pub(super) mod elastic;
/// The ways a pattern leaves the app, and the paper the printed ones take.
mod exports;
pub(super) mod fold;
pub(super) mod hang;
pub(super) mod inner;
pub(super) mod notch;
/// What one coordinate of a point reads as, and what it says instead when
/// the formula behind it will not evaluate.
mod reads;
pub(super) mod seams;
mod tape;
mod variables;
pub(super) mod write;

use cite::Cite;
use eframe::egui;
pub(super) use elastic::Grip;
use toile_engine::draft::{Axis, Draft, PieceKey, PointKey, SeamKey};
use toile_engine::session::SeamFault;
use write::Asked;

use super::curve::{self, Side};
use super::layout;
use super::state::{Scope, State};
use super::wire::Verb;
use crate::config::Paper;
use crate::theme::Theme;
use crate::widgets::{field_row, footer_note, section};

const NOTE: &str =
    "Las fórmulas se evalúan contra el maniquí elegido: mismo patrón, cualquier talla.";
const EMPTY: &str = "Carga una pieza desde el panel Producto para inspeccionarla.";

/// Room the footer keeps for itself under the scrolling body.
const FOOT_H: f32 = 74.0;

/// The right panel: the bindings of whatever is chosen, the names they can
/// read, and the ways out of the app.
///
/// It writes nothing itself; the edits it asks for are played by the tab, so
/// the document is borrowed for reading only while the panel draws. One field
/// confirmed is one named entry of the history, never a fold into whatever
/// gesture happened to be open; a rail dragged is one entry too, held open
/// across the frames of the drag. The names belong to the product and not to a
/// piece, so a product with no piece drawn yet lists them too. So do the
/// seams, which are listed over the whole product, where both sides of one
/// can be seen; `faults` are the ones the engine could not pair onto the cloth.
pub fn show(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: Option<&Draft>,
    faults: &[(SeamKey, SeamFault)],
    piece: Option<PieceKey>,
    state: &mut State,
    paper: Paper,
) -> Vec<Verb> {
    let body = (ui.available_height() - FOOT_H).max(0.0);
    let mut verbs = Vec::new();
    let asked = egui::ScrollArea::vertical()
        .max_height(body)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut asked = None;
            if let Some(draft) = draft {
                let cite = Cite::begin(ui.ctx(), draft.doc(), state);
                if let Some(piece) = piece {
                    asked = chosen(ui, theme, draft, piece, (state, &cite), &mut verbs);
                } else {
                    unchosen(ui, theme);
                }
                if state.scope == Scope::Product && piece.is_some() {
                    seams::show(ui, theme, draft, faults, (&mut *state, &mut verbs));
                }
                // The lines drawn inside a piece are listed where a piece is
                // open on its own, which is the one scope that edits them.
                if let Some(piece) = piece.filter(|_| state.scope == Scope::Piece) {
                    inner::show(ui, theme, draft, piece, (&mut *state, &mut verbs));
                }
                asked = tape::measures(ui, theme, draft, state, &cite).or(asked);
                asked = variables::variables(ui, theme, draft, state, &cite).or(asked);
            } else {
                unchosen(ui, theme);
            }
            exports::show(ui, theme, state, paper);
            asked
        })
        .inner;
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        footer_note(ui, theme, NOTE);
    });
    verbs.extend(elastic::settle(state));
    if let Some((label, command)) = asked {
        verbs.extend(super::entry(label, command));
    }
    verbs
}

/// What the panel says where a piece would be inspected and there is none.
fn unchosen(ui: &mut egui::Ui, theme: &Theme) {
    section(ui, theme, "Sin pieza");
    footer_note(ui, theme, EMPTY);
}

/// Whatever is chosen: a node, a group of them, a tract, or the piece.
fn chosen(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: &Draft,
    piece: PieceKey,
    writing: (&mut State, &Cite),
    verbs: &mut Vec<Verb>,
) -> Option<Asked> {
    let state = &*writing.0;
    if let Some(from) = state.selection.edge() {
        return tract(ui, theme, draft, (piece, from), writing, verbs);
    }
    match state.selection.count() {
        0 => {
            summary(ui, theme, draft, piece);
            None
        }
        1 => node(ui, theme, draft, piece, writing),
        many => {
            group(ui, theme, draft, state, many);
            None
        }
    }
}

/// The one chosen point, under the name the drawing gives it.
///
/// A handle is a point of the document like any other, so it gets the same two
/// rows; only the heading says which of the two it is, because "Punto" over a
/// tangent would leave the person guessing what they had hold of.
fn node(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: &Draft,
    piece: PieceKey,
    writing: (&mut State, &Cite),
) -> Option<Asked> {
    let point = writing.0.selection.only()?;
    section(ui, theme, &heading(draft, piece, point));
    write::coordinates(ui, theme, draft, (piece, point), writing)
}

/// What the panel calls the chosen point: a node by its name, a handle by the
/// node it pulls and the side it lies on.
fn heading(draft: &Draft, piece: PieceKey, point: PointKey) -> String {
    let doc = draft.doc();
    if let Some(name) = doc.label_of(piece, point) {
        return format!("Punto {name}");
    }
    let Some(hangs) = curve::hangs(doc, piece, point) else {
        return "Punto".to_owned();
    };
    let node = doc.label_of(piece, hangs.node).unwrap_or_default();
    let side = match hangs.side {
        Side::Out => "salida",
        Side::Into => "entrada",
    };
    format!("Manija de {side} · {node}")
}

/// Several nodes at once: what they have in common, and nothing they do not.
fn group(ui: &mut egui::Ui, theme: &Theme, draft: &Draft, state: &State, many: usize) {
    section(ui, theme, &format!("{many} puntos"));
    let doc = draft.doc();
    for (axis, label) in [(Axis::X, "X"), (Axis::Y, "Y")] {
        let mut sources = state.selection.points().map(|key| {
            doc.points
                .get(key)
                .map(|held| held.binding(axis).source().into_owned())
        });
        let first = sources.next().flatten();
        let common = match first {
            Some(source) if sources.all(|other| other.as_ref() == Some(&source)) => source,
            _ => "—".to_owned(),
        };
        field_row(ui, theme, label, &common, "");
    }
}

/// The chosen tract: the two nodes it runs between, how long it is, when it
/// bends how finely it is flattened, what holds it in, and what holds it up.
fn tract(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: &Draft,
    at: (PieceKey, PointKey),
    writing: (&mut State, &Cite),
    verbs: &mut Vec<Verb>,
) -> Option<Asked> {
    let (piece, from) = at;
    let nodes = draft.points_cm(piece);
    let index = nodes.iter().position(|&(key, _)| key == from)?;
    let to = nodes[(index + 1) % nodes.len()].0;
    let doc = draft.doc();
    let name = |key: PointKey| doc.label_of(piece, key).unwrap_or_default();
    section(ui, theme, &format!("Borde {} → {}", name(from), name(to)));
    let length = format!("{:.1}", draft.run_length_cm(piece, from, to));
    field_row(ui, theme, "largo", &length, "cm");
    let edge = (piece, from, to);
    let (state, cite) = writing;
    let asked = write::samples(ui, theme, draft, (piece, from), (&mut *state, cite));
    elastic::show(ui, theme, doc, edge, (&mut *state, &mut *verbs));
    hang::show(ui, theme, doc, edge, verbs);
    let asked = notch::show(ui, theme, doc, (piece, from), (&mut *state, cite), verbs).or(asked);
    fold::show(ui, theme, draft, edge, verbs);
    asked
}

/// What the piece is, when nothing on it is chosen.
///
/// Every measurement here is the cloth's and not the drawing's, so a piece
/// drawn against a fold reads as the piece that comes off the table.
fn summary(ui: &mut egui::Ui, theme: &Theme, draft: &Draft, piece: PieceKey) {
    let doc = draft.doc();
    let Some(held) = doc.pieces.get(piece) else {
        return;
    };
    section(ui, theme, &held.name);
    let nodes = held.contour.len().to_string();
    field_row(ui, theme, "nodos", &nodes, "");
    if let Some([left, top, right, bottom]) = layout::extent(draft.cloth_cm(piece)) {
        field_row(ui, theme, "ancho", &format!("{:.1}", right - left), "cm");
        field_row(ui, theme, "alto", &format!("{:.1}", bottom - top), "cm");
    }
    let perimeter = format!("{:.1}", draft.perimeter_cm(piece));
    field_row(ui, theme, "perímetro", &perimeter, "cm");
    let grain = format!("{:.0}", held.grain.radians().to_degrees());
    field_row(ui, theme, "hilo", &grain, "°");
    if draft.cloth(piece).is_some() {
        field_row(ui, theme, "doblez", "sí", "");
    }
}

#[cfg(test)]
mod tests;
