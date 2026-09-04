use eframe::egui;
use toile_engine::draft::{
    Axis, Command, Defect, Doc, Draft, EvalError, MannequinKey, PieceKey, PointKey,
};

use super::state::State;
use crate::theme::Theme;
use crate::widgets::{
    PAD, button_secondary, field_row, footer_note, formula_row, formula_row_fault, section,
    section_with, select,
};

const NOTE: &str =
    "Las fórmulas se evalúan contra el maniquí elegido: mismo patrón, cualquier talla.";
const EMPTY: &str = "Carga una pieza desde el panel Producto para inspeccionarla.";

/// Room the footer keeps for itself under the scrolling body.
const FOOT_H: f32 = 74.0;

/// The right panel: the bindings of whatever is chosen, the measurements they
/// resolve against, and the ways out of the app.
///
/// It writes nothing itself; the command it returns is applied by the tab, so
/// the document is borrowed for reading only while the panel draws.
pub fn show(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: Option<&Draft>,
    piece: Option<PieceKey>,
    state: &State,
) -> Option<Command> {
    let body = (ui.available_height() - FOOT_H).max(0.0);
    let command = egui::ScrollArea::vertical()
        .max_height(body)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let mut command = None;
            if let (Some(draft), Some(piece)) = (draft, piece) {
                chosen(ui, theme, draft, piece, state);
                command = measures(ui, theme, draft);
                variables(ui, theme, draft);
            } else {
                section(ui, theme, "Sin pieza");
                footer_note(ui, theme, EMPTY);
            }
            exports(ui, theme);
            command
        })
        .inner;
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        footer_note(ui, theme, NOTE);
    });
    command
}

/// The chosen node and its two bindings, or the piece when nothing is chosen.
fn chosen(ui: &mut egui::Ui, theme: &Theme, draft: &Draft, piece: PieceKey, state: &State) {
    let doc = draft.doc();
    let Some(point) = state.selection else {
        return summary(ui, theme, draft, piece);
    };
    let name = doc.label_of(piece, point).unwrap_or_default();
    section(ui, theme, &format!("Punto {name}"));
    let Some(held) = doc.points.get(point) else {
        return;
    };
    let at = draft.resolved(point);
    for (axis, label, k) in [(Axis::X, "X", 0), (Axis::Y, "Y", 1)] {
        let source = held.binding(axis).source();
        match at {
            Some(cm) => formula_row(ui, theme, label, &source, &format!("= {:.1} cm", cm[k])),
            None => formula_row_fault(ui, theme, label, &source, &why(draft, piece, point, axis)),
        }
    }
}

/// What the piece is, when no node is chosen.
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

/// The measurements the pattern resolves against, and the body it uses.
fn measures(ui: &mut egui::Ui, theme: &Theme, draft: &Draft) -> Option<Command> {
    let doc = draft.doc();
    let set = doc.measures()?;
    section_with(
        ui,
        theme,
        "Medidas del producto",
        &set.values.len().to_string(),
    );
    let mut command = None;
    ui.horizontal(|ui| {
        ui.add_space(PAD);
        if select(ui, theme, "resolver con", &set.name, 170.0).clicked() {
            command = next_body(doc).map(|mannequin| Command::ResolveWith { mannequin });
        }
    });
    ui.add_space(6.0);
    for (name, value) in &set.values {
        field_row(ui, theme, name, &format!("{value:.1}"), "cm");
    }
    command
}

/// The pattern's own quantities, at what they currently come to.
fn variables(ui: &mut egui::Ui, theme: &Theme, draft: &Draft) {
    let doc = draft.doc();
    section_with(ui, theme, "Variables", &doc.variables.len().to_string());
    for (_, variable) in doc.variables.iter() {
        let value = draft.env().value(&variable.name);
        let shown = value.map_or_else(|| "—".to_owned(), |value| format!("{value:.2}"));
        field_row(ui, theme, &variable.name, &shown, "");
    }
}

/// The ways a pattern leaves the app, both waiting on the phase that writes
/// files.
fn exports(ui: &mut egui::Ui, theme: &Theme) {
    section(ui, theme, "Exportar");
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.add_space(PAD);
        button_secondary(ui, theme, "PDF A4 · 1:1");
        button_secondary(ui, theme, "SVG");
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
