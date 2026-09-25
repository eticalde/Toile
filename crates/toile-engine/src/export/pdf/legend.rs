use std::fmt::Write;

use super::super::units::{CAPTION, MARGIN, MM_PER_CM, TITLE, number};
use super::grid::OVERLAP;
use super::ink::Ink;
use super::paper::{Paper, points};
use super::place::{Joins, Place};
use super::sheet::Frame;
use super::text::show;

/// The side of the calibration square, in millimetres.
///
/// Fifty and not a hundred. The square is paper the pattern cannot have, and it
/// is on every sheet: a hundred would take a third of the height of an A4 sheet
/// away from every one of them. What a hundred bought, a longer baseline for
/// the ruler, the sheet gets back for nothing by printing the piece's own two
/// sides beside it — and on a tiled piece those are the two sides of the whole
/// thing, which is the one measurement that proves the tiling as well as the
/// scale.
const CALIBRATION: f64 = 50.0;

/// Blank paper between the pattern and the band that proves the scale, in
/// millimetres.
const GAP: f64 = 10.0;

/// The leading of the legend beside the square, in millimetres.
///
/// One and a half times the caption it sets, which is what fits the line that
/// names the body beside the square without the square growing to hold it.
const LEADING: f64 = 4.5;

/// How much of a sheet's height goes to proving the scale, in millimetres.
pub(super) const BAND: f64 = CALIBRATION + GAP;

/// The calibration square, in the bottom left corner of the printable area.
///
/// Placed from the sheet's own corner and not from the pattern's box, so that
/// what a ruler reads does not depend on which piece or which sheet is on the
/// page. Its corner needs no turning around: a margin up from the bottom is a
/// margin up from the bottom in both ways of measuring a page.
pub(super) fn square(out: &mut String) {
    let corner = number(points(MARGIN));
    let side = number(points(CALIBRATION));
    let _ = writeln!(out, "{corner} {corner} {side} {side} re S");
}

/// What the sheet says, in the language its reader cuts in.
pub(super) fn says(out: &mut String, frame: &Frame, ink: &Ink, size: [f64; 2], place: &Place) {
    let paper = frame.paper();
    let (name, body) = (ink.name(), ink.body());
    let mut said = if place.total == 1 {
        alone(paper, size)
    } else {
        tiled(paper, size, place)
    };
    // Which body the pattern was resolved against, on the paper: forty loose
    // sheets of one garment say nothing about whom it was cut for, and the same
    // piece resolved against another body is another pattern at the same size
    // of paper. The drawing has nowhere to put it — one file carries every
    // piece of a document, and one sheet of paper carries one piece.
    if let Some(body) = body {
        said.insert(0, format!("Trazada para «{body}»."));
    }
    // The legend is set beside the square and must not run past it: the sheet
    // below is the pattern's, and the paper below that is a border most
    // printers will not print on.
    debug_assert!((said.len() + 2) as f64 * LEADING <= CALIBRATION);
    let across = MARGIN + CALIBRATION + GAP;
    let top = paper.height_mm - MARGIN - CALIBRATION + TITLE;
    let titled = if place.total == 1 {
        format!("Pieza «{name}»")
    } else {
        format!("Pieza «{name}» · hoja {} de {}", place.number, place.total)
    };
    show(out, frame.page(across, top), TITLE, &titled);
    for (rank, said) in said.iter().enumerate() {
        let down = top + LEADING * (rank + 1) as f64;
        show(out, frame.page(across, down), CAPTION, said);
    }
}

/// What a piece that fits on one sheet says: the scale, and nothing about
/// joining anything to anything.
fn alone(paper: Paper, size: [f64; 2]) -> Vec<String> {
    vec![
        "Escala 1:1 — imprime al 100 %.".to_owned(),
        "No uses «ajustar a la página».".to_owned(),
        format!("El cuadrado mide {} cm de lado.", CALIBRATION / MM_PER_CM),
        measured("La pieza", size),
        "Mide una de las dos con una regla antes de cortar.".to_owned(),
        named(paper),
    ]
}

/// What one sheet of a tiled piece says: where it is in the grid, which sheets
/// it is taped to, what to cut, and the scale.
fn tiled(paper: Paper, size: [f64; 2], place: &Place) -> Vec<String> {
    vec![
        format!(
            "Columna {} de {} · fila {} de {}.",
            place.cell[0] + 1,
            place.counts[0],
            place.cell[1] + 1,
            place.counts[1]
        ),
        beside(place.joins),
        format!(
            "Recorta por la línea de puntos y solapa {} cm.",
            OVERLAP / MM_PER_CM
        ),
        "Haz coincidir las cruces con las de la hoja vecina antes de pegar.".to_owned(),
        "Escala 1:1 — imprime al 100 %. No uses «ajustar a la página».".to_owned(),
        // Says the order and not only the length, because the line above tells
        // a person to cut along a band this square sits in: recorte first and
        // the proof of scale is in the bin before it was ever read. The sheet
        // that fits on one page says the same thing on its own line, where
        // there is room for it.
        format!(
            "El cuadrado mide {} cm de lado: mídelo antes de recortarlo.",
            CALIBRATION / MM_PER_CM
        ),
        measured("La pieza entera", size),
        named(paper),
    ]
}

/// Which sheets this one is taped to, named in one fixed order so that a person
/// holding two sheets reads the same words on both.
fn beside(joins: Joins) -> String {
    let sides = [
        ("arriba", joins.above),
        ("abajo", joins.below),
        ("izquierda", joins.left),
        ("derecha", joins.right),
    ];
    let said: Vec<String> = sides
        .iter()
        .filter_map(|&(side, sheet)| sheet.map(|sheet| format!("{side} {sheet}")))
        .collect();
    if said.is_empty() {
        return "Hojas vecinas: ninguna.".to_owned();
    }
    format!("Hojas vecinas: {}.", said.join(" · "))
}

/// What the piece measures, which is the longest thing on the sheet a ruler can
/// be laid along.
fn measured(what: &str, size: [f64; 2]) -> String {
    format!(
        "{what} mide {:.1} cm de ancho y {:.1} cm de alto.",
        size[0] / MM_PER_CM,
        size[1] / MM_PER_CM
    )
}

/// The paper the sheet was laid out for, so a person who reprints it on another
/// size sees that it was not this one.
fn named(paper: Paper) -> String {
    format!(
        "Papel {} · {} × {} mm.",
        paper.name, paper.width_mm, paper.height_mm
    )
}
