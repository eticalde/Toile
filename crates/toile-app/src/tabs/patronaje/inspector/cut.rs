use eframe::egui::{self, Id};
use toile_engine::draft::{Command, Doc, Piece, PieceKey};

use super::super::state::{Cut, Field, State};
use super::cite::Cite;
use super::write::{self, Asked};
use crate::theme::Theme;
use crate::widgets::{Editable, Edited, footer_note, formula_row, section, typed_row};

const ALLOWANCE: &str = "escribir el margen";
const QUANTITY: &str = "escribir cuántas se cortan";
const LETTER: &str = "escribir la letra";
const LABEL: &str = "escribir el rótulo";

const LETTER_NOTE: &str = "la del rótulo · vacío es sin letra";
const QUANTITY_NOTE: &str = "cuántas se cortan de esta pieza";
const OUTSIDE_NOTE: &str = "cm por fuera del contorno";
const NET_NOTE: &str = "neta: se corta por el contorno";
const SAYS: &str = "El margen va por fuera del contorno, y el contorno sigue siendo la línea de \
                    costura: la hoja lo dice, la mesa no lo traza. Un renglón vacío del rótulo se \
                    quita; los demás salen tal cual, sin que nadie les saque un recuento.";

/// What the piece says about being cut out, and the boxes that write it.
///
/// Nothing is written by looking: every box comes up holding what the document
/// holds, and a box confirmed on the value it came up with asks for nothing.
/// Opening a product and reading this section leaves its bytes and its version
/// exactly as they were.
pub fn show(
    ui: &mut egui::Ui,
    theme: &Theme,
    doc: &Doc,
    piece: PieceKey,
    (state, cite): (&mut State, &Cite),
) -> Option<Asked> {
    let held = doc.pieces.get(piece)?;
    section(ui, theme, "Corte");
    let mut asked = letter(ui, theme, (piece, held), (&mut *state, cite));
    asked = quantity(ui, theme, (piece, held), (&mut *state, cite)).or(asked);
    asked = allowance(ui, theme, (piece, held), (&mut *state, cite)).or(asked);
    asked = label(ui, theme, (piece, held), (&mut *state, cite)).or(asked);
    footer_note(ui, theme, SAYS);
    asked
}

/// One box of the section: its name, what it holds, which of the four it is.
struct Row<'a> {
    label: &'a str,
    source: &'a str,
    of: Cut,
}

/// The letter the piece's label shows, which is free text.
///
/// Whatever its writer put on the paper, and nothing here refuses a letter
/// another piece already shows: a letter is not a key, the name is the thing
/// the document keeps unique. An empty box is no letter at all, which is a
/// different claim from a letter nobody can read.
fn letter(
    ui: &mut egui::Ui,
    theme: &Theme,
    (piece, held): (PieceKey, &Piece),
    writing: (&mut State, &Cite),
) -> Option<Asked> {
    let source = held.letter.clone().unwrap_or_default();
    let row = Row {
        label: "letra",
        source: &source,
        of: Cut::Letter,
    };
    let written = box_of(ui, theme, piece, (&row, LETTER_NOTE), writing)?;
    let to = Some(written).filter(|text| !text.is_empty());
    Some((LETTER, Command::SetLetter { piece, to }))
}

/// How many of the piece the garment takes.
fn quantity(
    ui: &mut egui::Ui,
    theme: &Theme,
    (piece, held): (PieceKey, &Piece),
    writing: (&mut State, &Cite),
) -> Option<Asked> {
    let source = held.quantity.to_string();
    let row = Row {
        label: "cortar",
        source: &source,
        of: Cut::Quantity,
    };
    let written = box_of(ui, theme, piece, (&row, QUANTITY_NOTE), writing)?;
    let to = written.parse::<u32>().ok().filter(|&count| count > 0)?;
    Some((QUANTITY, Command::SetQuantity { piece, to }))
}

/// How far outside the drawn line the cloth is cut, in centimetres.
///
/// The line under the box is what the piece is rather than what may be typed,
/// because the two states of this one row are the two ways a piece is cut and
/// an empty box on its own does not say which: a piece nobody wrote a width
/// for reads as cut on its own line, and that is the sentence it carries.
fn allowance(
    ui: &mut egui::Ui,
    theme: &Theme,
    (piece, held): (PieceKey, &Piece),
    writing: (&mut State, &Cite),
) -> Option<Asked> {
    let source = held.seam_allowance.map(centimetres).unwrap_or_default();
    let row = Row {
        label: "margen",
        source: &source,
        of: Cut::Allowance,
    };
    let note = if source.is_empty() {
        NET_NOTE
    } else {
        OUTSIDE_NOTE
    };
    let written = box_of(ui, theme, piece, (&row, note), writing)?;
    let to = match width(&written)? {
        Width::Net => None,
        Width::Outside(cm) => Some(cm),
    };
    Some((ALLOWANCE, Command::SetSeamAllowance { piece, to }))
}

/// The lines of the label, in order, over an empty row for one more.
///
/// The whole label goes back at once, which is the shape the document keeps it
/// in. A line emptied is a line gone — the one way offered to take one off —
/// and the row holding nothing is where another goes on; the footer says both,
/// because a rule nobody can see is its own kind of lie. Each line is a row
/// with no line under it: nothing somebody writes here can be refused, so
/// there is nothing a note of its own would ever say.
fn label(
    ui: &mut egui::Ui,
    theme: &Theme,
    (piece, held): (PieceKey, &Piece),
    (state, cite): (&mut State, &Cite),
) -> Option<Asked> {
    let mut asked = None;
    for index in 0..=held.labels.len() {
        let source = held.labels.get(index).cloned().unwrap_or_default();
        let more = index == held.labels.len();
        let name = if more {
            "añadir".to_owned()
        } else {
            format!("rótulo {}", index + 1)
        };
        let row = Row {
            label: &name,
            source: &source,
            of: Cut::Label(index),
        };
        let Some(written) = line(ui, theme, piece, &row, (&mut *state, cite)) else {
            continue;
        };
        asked = Some((
            LABEL,
            Command::SetLabels {
                piece,
                to: lines(&held.labels, index, written),
            },
        ));
    }
    asked
}

/// The label `held` becomes when its line at `index` is written as `text`.
///
/// Past the end is a line added, and an empty text is the line at `index`
/// taken off; the empty box past the end confirms nothing, so the two never
/// meet.
fn lines(held: &[String], index: usize, text: String) -> Vec<String> {
    let mut lines = held.to_vec();
    match (text.is_empty(), index < lines.len()) {
        (true, true) => {
            lines.remove(index);
        }
        (true, false) => {}
        (false, true) => lines[index] = text,
        (false, false) => lines.push(text),
    }
    lines
}

/// Draws a box over the line that says what may go in it, for the rows whose
/// text the document can refuse.
fn box_of(
    ui: &mut egui::Ui,
    theme: &Theme,
    piece: PieceKey,
    (row, note): (&Row<'_>, &str),
    writing: (&mut State, &Cite),
) -> Option<String> {
    written(ui, piece, (row, note), writing, |ui, id, it| {
        formula_row(ui, theme, id, it)
    })
}

/// Draws a box on a row of its own height, with nothing under it.
fn line(
    ui: &mut egui::Ui,
    theme: &Theme,
    piece: PieceKey,
    row: &Row<'_>,
    writing: (&mut State, &Cite),
) -> Option<String> {
    written(ui, piece, (row, ""), writing, |ui, id, it| {
        typed_row(ui, theme, id, it)
    })
}

/// Hands back the text a box was confirmed with, when that text is not the one
/// the box came up holding.
///
/// Nothing is written until the text really changes. A box comes up with what
/// the document holds already in it, and a click in and out of a box confirms
/// what it holds, so without this an entry of the history would be opened for
/// a value nobody touched. The text is trimmed: a line of spaces is a line
/// that prints as nothing, and would read on the panel as a line that is gone.
fn written(
    ui: &mut egui::Ui,
    piece: PieceKey,
    (row, note): (&Row<'_>, &str),
    (state, cite): (&mut State, &Cite),
    draw: impl FnOnce(&mut egui::Ui, Id, &Editable<'_>) -> Edited,
) -> Option<String> {
    let shown = Editable {
        label: row.label,
        source: row.source,
        note,
        fault: false,
        held: None,
    };
    let text = write::field(ui, state, cite, Field::Cut(piece, row.of), &shown, draw)?;
    let text = text.trim();
    (text != row.source).then(|| text.to_owned())
}

/// The two ways a piece is cut out, which is what the margin box chooses
/// between.
#[derive(Debug, PartialEq)]
enum Width {
    /// Cut on its own line, which is what an empty box says.
    Net,
    /// Cut this far outside the line.
    Outside(f64),
}

/// Which of the two a piece of text asks for, when it asks for either.
///
/// Nothing at all is the net piece, a different answer from a width of zero:
/// one is a piece whose writer said nothing, the other is one cut on the very
/// line it is sewn on. Text the document would refuse is read again here
/// rather than trusted — the row judged it before handing it over, so this is
/// the second lock on the one door.
fn width(text: &str) -> Option<Width> {
    if text.is_empty() {
        return Some(Width::Net);
    }
    let width = text.parse::<f64>().ok()?;
    (width.is_finite() && width >= 0.0).then_some(Width::Outside(width))
}

/// A width as the box shows it, in centimetres.
///
/// Two decimals, which is a tenth of a millimetre: finer than any hand cuts
/// to, and the box is the only thing that writes this number, so what the file
/// holds is what somebody typed. The zeros are trimmed, so the widths of the
/// trade read as the numbers somebody would type: `1.5` and `1`, never `1.50`.
fn centimetres(width: f64) -> String {
    let text = format!("{width:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

#[cfg(test)]
mod tests;
