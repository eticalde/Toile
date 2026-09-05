mod write;

use eframe::egui;
use toile_engine::draft::{Axis, Defect, Doc, Draft, EvalError, MannequinKey, PieceKey, PointKey};
use write::Asked;

use super::curve::{self, Side};
use super::state::State;
use crate::file::Action;
use crate::theme::Theme;
use crate::widgets::{button_ghost, button_secondary, field_row, footer_note, section};

const NOTE: &str =
    "Las fórmulas se evalúan contra el maniquí elegido: mismo patrón, cualquier talla.";
const EMPTY: &str = "Carga una pieza desde el panel Producto para inspeccionarla.";

/// Room the footer keeps for itself under the scrolling body.
const FOOT_H: f32 = 74.0;

/// The right panel: the bindings of whatever is chosen, the measurements they
/// resolve against, and the ways out of the app.
///
/// It writes nothing itself; the edit it asks for is applied by the tab, so
/// the document is borrowed for reading only while the panel draws. One field
/// confirmed is one named entry of the history, never a fold into whatever
/// gesture happened to be open.
pub fn show(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: Option<&Draft>,
    piece: Option<PieceKey>,
    state: &mut State,
) -> Option<Asked> {
    let body = (ui.available_height() - FOOT_H).max(0.0);
    let asked = egui::ScrollArea::vertical()
        .max_height(body)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut asked = None;
            if let (Some(draft), Some(piece)) = (draft, piece) {
                asked = chosen(ui, theme, draft, piece, state);
                asked = write::measures(ui, theme, draft, state).or(asked);
                asked = write::variables(ui, theme, draft, state).or(asked);
            } else {
                section(ui, theme, "Sin pieza");
                footer_note(ui, theme, EMPTY);
            }
            exports(ui, theme, state);
            asked
        })
        .inner;
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        footer_note(ui, theme, NOTE);
    });
    asked
}

/// Whatever is chosen: a node, a group of them, a tract, or the piece.
fn chosen(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: &Draft,
    piece: PieceKey,
    state: &mut State,
) -> Option<Asked> {
    if let Some(from) = state.selection.edge() {
        return tract(ui, theme, draft, (piece, from), state);
    }
    match state.selection.count() {
        0 => {
            summary(ui, theme, draft, piece);
            None
        }
        1 => node(ui, theme, draft, piece, state),
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
    state: &mut State,
) -> Option<Asked> {
    let point = state.selection.only()?;
    section(ui, theme, &heading(draft, piece, point));
    write::coordinates(ui, theme, draft, (piece, point), state)
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

/// The chosen tract: the two nodes it runs between, how long it is, and — when
/// it bends — how finely it is flattened.
fn tract(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: &Draft,
    at: (PieceKey, PointKey),
    state: &mut State,
) -> Option<Asked> {
    let (piece, from) = at;
    let nodes = draft.points_cm(piece);
    let index = nodes.iter().position(|&(key, _)| key == from)?;
    let to = nodes[(index + 1) % nodes.len()].0;
    let doc = draft.doc();
    let ends = (
        doc.label_of(piece, from).unwrap_or_default(),
        doc.label_of(piece, to).unwrap_or_default(),
    );
    section(ui, theme, &format!("Borde {} → {}", ends.0, ends.1));
    let length = format!("{:.1}", draft.run_length_cm(piece, from, to));
    field_row(ui, theme, "largo", &length, "cm");
    write::samples(ui, theme, draft, (piece, from), state)
}

/// What the piece is, when nothing on it is chosen.
fn summary(ui: &mut egui::Ui, theme: &Theme, draft: &Draft, piece: PieceKey) {
    let doc = draft.doc();
    let Some(held) = doc.pieces.get(piece) else {
        return;
    };
    section(ui, theme, &held.name);
    let nodes = held.contour.len().to_string();
    field_row(ui, theme, "nodos", &nodes, "");
    let perimeter = format!("{:.1}", draft.perimeter_cm(piece));
    field_row(ui, theme, "perímetro", &perimeter, "cm");
    let grain = format!("{:.0}", held.grain.radians().to_degrees());
    field_row(ui, theme, "hilo", &grain, "°");
}

/// The ways a pattern leaves the app: the drawing at true scale, and the
/// sheets of paper it is tiled onto, which wait on the phase that lays them
/// out and is drawn dead until then.
fn exports(ui: &mut egui::Ui, theme: &Theme, state: &mut State) {
    section(ui, theme, "Exportar");
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.add_space(crate::widgets::PAD);
        button_ghost(ui, theme, "PDF A4 · 1:1");
        if button_secondary(ui, theme, "SVG").clicked() {
            state.asked = Some(Action::Svg);
        }
    });
}

/// The next body in the document, when there is another one to try.
fn next_body(doc: &Doc) -> Option<MannequinKey> {
    let bodies: Vec<MannequinKey> = doc.mannequins.keys().collect();
    if bodies.len() < 2 {
        return None;
    }
    let at = bodies.iter().position(|&key| key == doc.resolve_with)?;
    bodies.get((at + 1) % bodies.len()).copied()
}

/// Why a coordinate does not resolve, said in the language of the panel.
fn why(draft: &Draft, piece: PieceKey, point: PointKey, axis: Axis) -> String {
    let found = draft.defects(piece).iter().find_map(|defect| match defect {
        Defect::Binding {
            point: at,
            axis: which,
            error,
        } if *at == point && *which == axis => Some(error),
        _ => None,
    });
    match found {
        Some(EvalError::UnknownName(name)) => format!("nombre desconocido: {name}"),
        Some(EvalError::DivideByZero) => "división por cero".to_owned(),
        Some(EvalError::FractionalPower) => "el exponente no es entero".to_owned(),
        Some(EvalError::NotFinite) => "no da un número".to_owned(),
        Some(EvalError::Cycle(names)) => format!("variables circulares: {names}"),
        None => "no resuelve".to_owned(),
    }
}
