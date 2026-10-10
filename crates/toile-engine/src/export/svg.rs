/// Every run of ink inside a piece, as a path.
mod ink;
mod mark;
/// Text as XML takes it, which is the one part of the drawing's dialect the
/// printed sheet cannot share: a PDF escapes its own strings its own way.
mod xml;

use std::fmt::Write;

use super::block::{self, Room};
use super::drawn::{self, Drawn};
use super::metric;
use super::units::{self, CAPTION, CUT, INK, MARGIN, beside, box_of, number};
use crate::draft::{Draft, Piece, PieceKey};

/// What stops a pattern from being written as a drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ExportError {
    /// Nothing in the document resolves to a contour a sheet could carry.
    #[error("the pattern draws no piece that resolves to a contour")]
    Empty,
}

/// A pattern drawn once, and what the drawing could not say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drawing {
    /// The drawing itself.
    pub text: String,
    /// How many pieces found nowhere on it to say their own words.
    ///
    /// The piece that lost its label also says so in its own group's title,
    /// which is what a person hovering it reads. This is for the door they
    /// came in by: a drawing carries no legend to put a count in, and a title
    /// nobody opens is a warning nobody gets.
    pub unsaid: usize,
}

/// The document as an SVG drawing at true scale.
///
/// One user unit is one millimetre and the sheet declares itself in
/// millimetres, so the drawing measures on a ruler what the pattern says it
/// measures: the side seam of the base block is 104.6 cm in any program that
/// reads SVG. Each piece is one group — the line it is cut on and everything
/// drawn inside it — so a piece can be moved or hidden on its own.
///
/// The words a piece carries are laid out against the drawing, against the
/// other pieces on it and against every block already placed, the same way the
/// printed sheet lays them out against its own cell: a drawing is one page, so
/// a block on it is never clipped, and a block that landed inside the piece
/// beside it or on top of its neighbour's words would be as anonymous here as
/// it is on paper.
///
/// # Errors
/// `ExportError::Empty` when no piece of the document resolves to a contour.
pub fn to_svg(draft: &Draft) -> Result<Drawing, ExportError> {
    let pieces: Vec<PieceKey> = draft
        .doc()
        .piece_keys()
        .into_iter()
        .filter(|&piece| draft.points_cm(piece).len() >= 3)
        .collect();
    let cloths: Vec<Vec<[f64; 2]>> = pieces
        .iter()
        .map(|&piece| millimetres(draft, piece))
        .collect();
    let sheet = sheet(&cloths).ok_or(ExportError::Empty)?;
    let inked: Vec<Drawn> = pieces
        .iter()
        .map(|&piece| drawn::of(draft, piece))
        .collect();
    let mut taken = named(&inked);
    let mut unsaid = 0;
    let mut out = String::new();
    header(&mut out, sheet);
    for (rank, &piece) in pieces.iter().enumerate() {
        let Some(held) = draft.doc().pieces.get(piece) else {
            continue;
        };
        let others: Vec<&[[f64; 2]]> = cloths
            .iter()
            .enumerate()
            .filter(|&(other, _)| other != rank)
            .map(|(_, cloth)| cloth.as_slice())
            .collect();
        let room = Room {
            sheet: [sheet[0], sheet[1], sheet[0] + sheet[2], sheet[1] + sheet[3]],
            cloth: &cloths[rank],
            taken: &taken,
            others: &others,
        };
        // Laid out before the group is written, because a piece that could not
        // say its own words anywhere says that in its own title — and because
        // the piece after it has to be told where these words went.
        let said = mark::said(held, &room);
        unsaid += usize::from(said.is_none());
        group(&mut out, held, &cloths[rank], &inked[rank], said.as_deref());
        taken.extend(
            said.unwrap_or_default()
                .iter()
                .map(|line| metric::box_of(line.size, line.at, &line.body)),
        );
    }
    out.push_str("</svg>\n");
    Ok(Drawing { text: out, unsaid })
}

/// The sheet the pieces fit on: its corner and its size, in millimetres.
///
/// `None` when there is nothing on it, which is the one thing a drawing cannot
/// be made of.
fn sheet(cloths: &[Vec<[f64; 2]>]) -> Option<[f64; 4]> {
    let (low, high) = box_of(cloths.iter().flatten().copied());
    if low[0] > high[0] {
        return None;
    }
    Some([
        low[0] - MARGIN,
        low[1] - MARGIN,
        high[0] - low[0] + 2.0 * MARGIN,
        high[1] - low[1] + 2.0 * MARGIN,
    ])
}

/// The box every node name of the drawing takes, of every piece on it.
///
/// Gathered before a single block is placed, because a block that dodged only
/// its own piece's names would land on its neighbour's.
fn named(inked: &[Drawn]) -> Vec<[f64; 4]> {
    inked
        .iter()
        .flat_map(|drawn| drawn.names.iter())
        .map(|(label, at)| metric::box_of(CAPTION, beside(units::millimetres(*at)), label))
        .collect()
}

/// The opening of the file, where the true scale is declared.
fn header(out: &mut String, sheet: [f64; 4]) {
    let [x, y, w, h] = sheet.map(number);
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = writeln!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}mm\" height=\"{h}mm\" \
         viewBox=\"{x} {y} {w} {h}\">"
    );
}

/// One piece: its cut line, and everything drawn inside it.
fn group(
    out: &mut String,
    piece: &Piece,
    cloth: &[[f64; 2]],
    drawn: &Drawn,
    said: Option<&[block::Line]>,
) {
    let _ = writeln!(out, "  <g>");
    let _ = writeln!(
        out,
        "    <title>{}</title>",
        mark::titled(piece, said.is_some())
    );
    contour(out, cloth);
    ink::runs(out, &drawn.runs);
    mark::names(out, &drawn.names, said);
    let _ = writeln!(out, "  </g>");
}

/// The cut line: one closed path, in contour order.
fn contour(out: &mut String, outline: &[[f64; 2]]) {
    let mut path = String::new();
    for (rank, &[x, y]) in outline.iter().enumerate() {
        let verb = if rank == 0 { 'M' } else { 'L' };
        let _ = write!(path, "{verb} {} {} ", number(x), number(y));
    }
    path.push('Z');
    let _ = writeln!(
        out,
        "    <path d=\"{path}\" fill=\"none\" stroke=\"{INK}\" stroke-width=\"{}\"/>",
        number(CUT)
    );
}

/// A piece's cut line in millimetres: the cloth, with its curves flattened.
///
/// The cloth and not the drawing, so a piece drawn against a fold is printed
/// whole. On a sheet carrying the half, the cut line and the net line are a
/// centimetre apart and nothing says which one the fold is laid on.
fn millimetres(draft: &Draft, piece: PieceKey) -> Vec<[f64; 2]> {
    draft
        .cloth_cm(piece)
        .iter()
        .copied()
        .map(units::millimetres)
        .collect()
}

#[cfg(test)]
mod tests;
