/// Which sheets a piece takes, in which order, and which of them are blank.
mod grid;
/// Everything one piece puts on paper apart from the line it is cut on.
mod ink;
/// What a person needs to put one sheet against the next.
mod join;
/// Whether any of the cloth falls on one cell of paper.
mod lands;
/// The square a ruler is laid on, and the Spanish beside it.
mod legend;
#[cfg(test)]
mod marked;
/// The two paper sizes, and the transform from a document centimetre to a
/// page point.
mod paper;
/// Which sheet of a piece one sheet is.
mod place;
/// The plane the taped sheets make, and where a place of the piece falls on it.
mod plane;
/// The drawing on one sheet: where it sits, and everything inked on it.
mod sheet;
#[cfg(test)]
mod tests;
/// A line of Spanish as a string a reader takes without guessing.
mod text;
#[cfg(test)]
mod tiled;

use self::grid::Grid;
use self::ink::Ink;
pub use self::paper::{A4, CARTA, Paper};
use self::sheet::{Frame, content};
use super::drawn::Inked;
use super::units::number;
use crate::draft::{Draft, PieceKey};

/// The id each object of the file is written under.
///
/// Fixed, and not handed out while walking a collection: an id that came out
/// of a hash map's iteration order would make the same document a different
/// file on the next run, for no reason a reader of either could see.
const CATALOG: usize = 1;
const PAGES: usize = 2;
const FONT: usize = 3;

/// The two ids one sheet takes: its page, and what is drawn on it.
///
/// Arithmetic on the sheet's own rank, so a piece that takes forty sheets
/// numbers them the same way twice, and the fifth sheet of one run is the
/// fifth sheet of the next.
fn ids(rank: usize) -> [usize; 2] {
    [FONT + 1 + 2 * rank, FONT + 2 + 2 * rank]
}

/// What stops a piece from being written as printable sheets.
#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
pub enum SheetError {
    /// The piece resolves to no contour a sheet could carry.
    #[error("the piece resolves to no contour a sheet could carry")]
    Empty,
    /// The piece would be laid across more paper than a ream.
    ///
    /// Said in the centimetres the document counts in as well as in sheets: the
    /// person reading it is holding a pattern, and the number that tells them
    /// which formula slipped is the size, not the sheet count.
    #[error(
        "the piece measures {:.1} by {:.1} cm, which is {sheets} sheets of {paper}",
        size_cm[0], size_cm[1]
    )]
    TooMany {
        /// How many sheets the piece would be laid across.
        sheets: usize,
        /// What the piece measures, in centimetres.
        size_cm: [f64; 2],
        /// What that paper is called.
        paper: &'static str,
    },
}

/// One piece as sheets of paper, at true scale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Printed {
    /// The file, of one page per sheet.
    pub bytes: Vec<u8>,
    /// How many sheets it prints.
    pub sheets: usize,
    /// How many columns and rows the piece was laid across, blank cells and
    /// all.
    pub grid: [usize; 2],
    /// How much of the piece besides its cut line reached the paper.
    pub inked: Inked,
}

impl Printed {
    /// How many cells of the grid carry no cloth and are therefore not printed.
    #[must_use]
    pub fn blank(&self) -> usize {
        self.grid[0] * self.grid[1] - self.sheets
    }
}

/// One piece as sheets to print at true scale, tiled across the paper it takes.
///
/// Everything the pattern draws on the piece is on them: the line it is cut on,
/// the lines drawn inside it, its notches, its grain and its names.
///
/// The page box is each sheet's own paper size, so printing at 100 % is the
/// identity and a viewer's fit-to-page is the only thing left that can lie
/// about the scale, which is why every sheet says in Spanish not to use it. A
/// piece larger than the paper is tiled and never shrunk: a trouser leg is most
/// of a metre long and no sheet of paper is.
///
/// No clock reaches the file and no object id depends on anything but a sheet's
/// rank, so the bytes are a function of the document alone.
///
/// # Errors
/// `SheetError::Empty` when the piece resolves to no contour, and
/// `SheetError::TooMany` when it would take more paper than a ream.
pub fn to_pdf(draft: &Draft, piece: PieceKey, paper: Paper) -> Result<Printed, SheetError> {
    let outline = draft.cloth_cm(piece);
    if outline.len() < 3 {
        return Err(SheetError::Empty);
    }
    let grid = Grid::new(paper, outline)?;
    let ink = Ink::new(draft, piece);
    // Every node of the cloth is inside the box the grid is built from, so a
    // piece that resolves to a contour always lands on at least one sheet.
    let streams: Vec<String> = grid
        .places()
        .iter()
        .map(|place| {
            let frame = Frame::new(paper, grid.corner(place.cell));
            content(&frame, &grid, &ink, place)
        })
        .collect();
    Ok(Printed {
        sheets: streams.len(),
        grid: grid.counts(),
        inked: ink.inked(),
        bytes: file(&objects(paper, &streams)),
    })
}

/// The objects the file is made of, in the order they are written.
///
/// A catalogue, a page tree, the one font they all name, and then two objects
/// per sheet. Helvetica is one of the fourteen every reader carries, so nothing
/// has to be embedded and the file stays what it draws.
fn objects(paper: Paper, streams: &[String]) -> Vec<String> {
    let [width, height] = paper.points();
    let kids: Vec<String> = (0..streams.len())
        .map(|rank| format!("{} 0 R", ids(rank)[0]))
        .collect();
    let mut out = vec![
        format!("<< /Type /Catalog /Pages {PAGES} 0 R >>"),
        format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            kids.join(" "),
            streams.len()
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_owned(),
    ];
    for (rank, stream) in streams.iter().enumerate() {
        let [_, drawn] = ids(rank);
        out.push(format!(
            "<< /Type /Page /Parent {PAGES} 0 R /MediaBox [0 0 {} {}] /Resources << /Font << /F1 \
             {FONT} 0 R >> >> /Contents {drawn} 0 R >>",
            number(width),
            number(height)
        ));
        out.push(format!(
            "<< /Length {} >>\nstream\n{stream}endstream",
            stream.len()
        ));
    }
    out
}

/// The objects wrapped in a file: the header, the table that says where each
/// object starts, and the trailer that says where that table is.
///
/// Every entry of the table is the twenty bytes the format asks for — ten
/// digits of offset, five of generation, the kind, and a two-byte end of line —
/// because a reader that finds an object one byte from where the table sent it
/// refuses the file rather than looking for it.
fn file(objects: &[String]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n");
    // Four bytes above 127 on the second line, so that a tool which has to
    // guess tells this file from a text one and stops translating newlines.
    out.extend_from_slice(&[b'%', 0xe2, 0xe3, 0xcf, 0xd3, b'\n']);
    let mut offsets = Vec::with_capacity(objects.len());
    for (rank, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", rank + 1).as_bytes());
    }
    let table = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f\r\n");
    for at in offsets {
        out.extend_from_slice(format!("{at:010} 00000 n\r\n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root {CATALOG} 0 R >>\nstartxref\n{table}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}
