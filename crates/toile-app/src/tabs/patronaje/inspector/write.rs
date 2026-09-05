use eframe::egui::{self, Id};
use toile_engine::draft::{Axis, Binding, Command, Draft, PieceKey, PointKey, SyntaxError};

use super::super::curve;
use super::super::state::{Field, FieldEdit, State};
use crate::theme::Theme;
use crate::widgets::{Editable, Edited, PAD, formula_row, section_with, select};

/// One edit a panel asks for, under the name it will carry in the history.
pub type Asked = (&'static str, Command);

const BIND: &str = "escribir fórmula";
const SAMPLES: &str = "afinar el aplanado";
const MEASURE: &str = "editar medida";
const VARIABLE: &str = "editar variable";
const BODY: &str = "cambiar de cuerpo";

/// The chosen node and its two bindings, each over what it comes to.
///
/// The formulas are read straight from the document, so a drag that rewrites
/// an adjustment term shows it here in the same frame it writes it. What is
/// typed into a box, on the other hand, is nobody's business until it parses:
/// the geometry is not touched until the row is confirmed.
pub fn coordinates(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: &Draft,
    at: (PieceKey, PointKey),
    state: &mut State,
) -> Option<Asked> {
    let (piece, point) = at;
    let held = draft.doc().points.get(point)?;
    let resolved = draft.resolved(point);
    let mut asked = None;
    for (axis, label, k) in [(Axis::X, "X", 0), (Axis::Y, "Y", 1)] {
        let source = held.binding(axis).source().into_owned();
        let note = match resolved {
            Some(cm) => format!("= {:.1} cm", cm[k]),
            None => super::why(draft, piece, point, axis),
        };
        let written = row(
            ui,
            theme,
            state,
            Field::Coordinate(point, axis),
            &Editable {
                label,
                source: &source,
                note: &note,
                fault: resolved.is_none(),
                held: None,
            },
        );
        if let Some(text) = written
            && let Ok(to) = Binding::parse(text.trim())
        {
            asked = Some((BIND, Command::SetBinding { point, axis, to }));
        }
    }
    asked
}

/// How finely the chosen tract is flattened, when it is one that bends.
///
/// A straight tract has no row: its sample count is a number the flattening
/// never reads, and a field that changes nothing is a field that lies.
pub fn samples(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: &Draft,
    at: (PieceKey, PointKey),
    state: &mut State,
) -> Option<Asked> {
    let (piece, node) = at;
    let held = curve::samples_of(draft.doc(), piece, node)?;
    let source = held.to_string();
    let (low, high) = curve::SAMPLE_RANGE;
    let written = row(
        ui,
        theme,
        state,
        Field::Samples(node),
        &Editable {
            label: "muestras",
            source: &source,
            note: &format!("puntos del aplanado · {low} a {high}"),
            fault: false,
            held: None,
        },
    );
    let to = counted(written.as_deref()?)?;
    Some((SAMPLES, Command::SetSamples { piece, node, to }))
}

/// The sample count a piece of text asks for, when it asks for a usable one.
///
/// Out of range is refused here and not clamped downstream: a tract flattened
/// to more points than a whole piece is meshed with is a typo, and building it
/// before saying so would spend the memory to prove the point.
fn counted(text: &str) -> Option<u16> {
    let (low, high) = curve::SAMPLE_RANGE;
    text.trim().parse::<u16>().ok().filter(|&to| {
        let range = low..=high;
        range.contains(&to)
    })
}

/// The measurements the pattern resolves against, and the body it uses.
pub fn measures(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: &Draft,
    state: &mut State,
) -> Option<Asked> {
    let doc = draft.doc();
    let set = doc.measures()?;
    let mannequin = doc.resolve_with;
    section_with(
        ui,
        theme,
        "Medidas del producto",
        &set.values.len().to_string(),
    );
    let mut asked = None;
    ui.horizontal(|ui| {
        ui.add_space(PAD);
        if select(ui, theme, "resolver con", &set.name, 170.0).clicked() {
            asked = super::next_body(doc).map(|to| (BODY, Command::ResolveWith { mannequin: to }));
        }
    });
    ui.add_space(6.0);
    let names: Vec<(String, f64)> = set
        .values
        .iter()
        .map(|(name, &value)| (name.clone(), value))
        .collect();
    for (name, value) in names {
        let source = format!("{value:.1}");
        let written = row(
            ui,
            theme,
            state,
            Field::Measure(name.clone()),
            &Editable {
                label: &name,
                source: &source,
                note: "cm",
                fault: false,
                held: None,
            },
        );
        if let Some(text) = written
            && let Ok(to) = text.trim().parse::<f64>()
            && to.is_finite()
        {
            asked = Some((
                MEASURE,
                Command::SetMeasure {
                    mannequin,
                    name,
                    to,
                },
            ));
        }
    }
    asked
}

/// The pattern's own quantities, at what they currently come to.
pub fn variables(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: &Draft,
    state: &mut State,
) -> Option<Asked> {
    let doc = draft.doc();
    section_with(ui, theme, "Variables", &doc.variables.len().to_string());
    let held: Vec<_> = doc
        .variables
        .iter()
        .map(|(key, variable)| {
            let value = draft.env().value(&variable.name);
            (
                key,
                variable.name.clone(),
                variable.value.source().into_owned(),
                value.map_or_else(|| "—".to_owned(), |value| format!("= {value:.2}")),
            )
        })
        .collect();
    let mut asked = None;
    for (key, name, source, note) in held {
        let written = row(
            ui,
            theme,
            state,
            Field::Variable(key),
            &Editable {
                label: &name,
                source: &source,
                note: &note,
                fault: false,
                held: None,
            },
        );
        if let Some(text) = written
            && let Ok(to) = Binding::parse(text.trim())
        {
            asked = Some((VARIABLE, Command::SetVariable { variable: key, to }));
        }
    }
    asked
}

/// Draws one editable row and hands back the text it was confirmed with.
///
/// The buffer belongs to the tab, so a row that has the focus paints whatever
/// has been typed into it, faults and all, while the document keeps the last
/// thing that parsed.
fn row(
    ui: &mut egui::Ui,
    theme: &Theme,
    state: &mut State,
    of: Field,
    shown: &Editable<'_>,
) -> Option<String> {
    let buffer = state
        .editing
        .as_ref()
        .filter(|edit| edit.of == of)
        .map(|edit| edit.buffer.clone());
    let fault = buffer.as_deref().and_then(|text| unparsed(&of, text));
    let note = fault.clone().unwrap_or_else(|| shown.note.to_owned());
    let row = Editable {
        note: &note,
        fault: fault.is_some() || shown.fault,
        held: buffer.as_deref(),
        ..*shown
    };
    match formula_row(ui, theme, id_of(&of), &row) {
        Edited::Idle => None,
        Edited::Typing(text) => {
            state.editing = Some(FieldEdit { of, buffer: text });
            None
        }
        Edited::Done(text) => {
            // Text that does not parse is kept, not thrown away, so the row
            // goes on painting the fault and the missed character can be
            // fixed. Nothing reaches the document until it parses.
            if unparsed(&of, &text).is_some() {
                state.editing = Some(FieldEdit { of, buffer: text });
                return None;
            }
            state.editing = None;
            Some(text)
        }
    }
}

/// Why the text in a box does not parse, said where it stops parsing.
///
/// A measurement is a number and nothing else: it is the body that carries it,
/// and a body measured by a formula is a pattern pretending to be a person.
fn unparsed(of: &Field, text: &str) -> Option<String> {
    let text = text.trim();
    match of {
        Field::Measure(_) => match text.parse::<f64>() {
            Ok(value) if value.is_finite() => None,
            _ => Some("no es un número".to_owned()),
        },
        Field::Samples(_) => {
            let (low, high) = curve::SAMPLE_RANGE;
            counted(text).map_or_else(|| Some(format!("un entero entre {low} y {high}")), |_| None)
        }
        Field::Coordinate(..) | Field::Variable(_) => {
            let fault: SyntaxError = Binding::parse(text).err()?;
            Some(format!("no parsea en {}: {}", fault.at, fault.kind))
        }
    }
}

/// The identity egui keeps a row's focus under.
///
/// It names the field and not the row's position, so the focus follows the
/// coordinate rather than the place on the panel it happens to be drawn at.
fn id_of(of: &Field) -> Id {
    Id::new(("patronaje-field", of))
}
