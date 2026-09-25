use eframe::egui::{self, Id};
use toile_engine::draft::{Axis, Binding, Command, Draft, PieceKey, PointKey, SyntaxError};

use super::super::curve;
use super::super::state::{Field, FieldEdit, State};
use super::cite::Cite;
use crate::theme::Theme;
use crate::widgets::{Editable, Edited, formula_row};

/// One edit a panel asks for, under the name it will carry in the history.
pub type Asked = (&'static str, Command);

const BIND: &str = "escribir fórmula";
const SAMPLES: &str = "afinar el aplanado";

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
    (state, cite): (&mut State, &Cite),
) -> Option<Asked> {
    let (piece, point) = at;
    let held = draft.doc().points.get(point)?;
    let mut asked = None;
    for (axis, label) in [(Axis::X, "X"), (Axis::Y, "Y")] {
        let source = held.binding(axis).source().into_owned();
        let (note, fault) = super::reads::coordinate(draft, (piece, point), axis);
        let shown = Editable {
            label,
            source: &source,
            note: &note,
            fault,
            held: None,
        };
        let written = field(
            ui,
            state,
            cite,
            Field::Coordinate(point, axis),
            &shown,
            |ui, id, row| formula_row(ui, theme, id, row),
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
    (state, cite): (&mut State, &Cite),
) -> Option<Asked> {
    let (piece, node) = at;
    let held = curve::samples_of(draft.doc(), piece, node)?;
    let source = held.to_string();
    let (low, high) = curve::SAMPLE_RANGE;
    let note = format!("puntos del aplanado · {low} a {high}");
    let shown = Editable {
        label: "muestras",
        source: &source,
        note: &note,
        fault: false,
        held: None,
    };
    let written = field(
        ui,
        state,
        cite,
        Field::Samples(node),
        &shown,
        |ui, id, row| formula_row(ui, theme, id, row),
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

/// Draws one editable row with `draw` and hands back the text it was
/// confirmed with.
///
/// The buffer belongs to the tab, so a row that has the focus paints whatever
/// has been typed into it, faults and all, while the document keeps the last
/// thing that parsed.
pub fn field(
    ui: &mut egui::Ui,
    state: &mut State,
    cite: &Cite,
    of: Field,
    shown: &Editable<'_>,
    draw: impl FnOnce(&mut egui::Ui, Id, &Editable<'_>) -> Edited,
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
    let id = id_of(&of);
    match draw(ui, id, &row) {
        Edited::Idle => None,
        Edited::Typing(text) => {
            state.editing = Some(FieldEdit { of, buffer: text });
            None
        }
        Edited::Done(text) => {
            // The focus went to a name pressed for this very text: the name
            // goes in and the person goes on writing, so nothing is confirmed
            // and the box takes the focus straight back.
            if cite.keeps(&of) {
                state.editing = Some(FieldEdit { of, buffer: text });
                ui.memory_mut(|m| m.request_focus(id));
                return None;
            }
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
        // A fraction of a tract, in the percentage the row shows: past either
        // end there is no tract left for the mark to sit on, and the anchor the
        // document takes would be refused on arrival.
        Field::Along(_) => match text.parse::<f64>() {
            Ok(value) if (0.0..=100.0).contains(&value) => None,
            _ => Some("un porcentaje entre 0 y 100".to_owned()),
        },
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
pub fn id_of(of: &Field) -> Id {
    Id::new(("patronaje-field", of))
}
