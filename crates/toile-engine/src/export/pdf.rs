/// Which sheets a piece takes, in which order, and which of them are blank.
mod grid;
/// Everything one piece puts on paper apart from the line it is cut on.
mod ink;
/// What a person needs to put one sheet against the next.
mod join;
/// The square a ruler is laid on, and the Spanish beside it.
mod legend;
#[cfg(test)]
mod marked;
/// Which pieces share a plane, and where each of them sits on it.
mod pack;
/// The two paper sizes, and the transform from a document centimetre to a
/// page point.
mod paper;
/// Which sheet of a piece one sheet is.
mod place;
/// The plane the taped sheets make, and where a place of the piece falls on it.
mod plane;
/// What one file holds: every piece of a product, or one piece of it.
mod product;
/// A written file read back the way a program that reads it would. Nothing in
/// it knows how the file was written: a proof of scale that asked the transform
/// to agree with itself would prove nothing.
#[cfg(test)]
mod read;
#[cfg(test)]
mod says;
/// The drawing on one sheet: where it sits, and everything inked on it.
mod sheet;
#[cfg(test)]
mod tests;
/// A line of Spanish as a string a reader takes without guessing.
mod text;
#[cfg(test)]
mod tiled;
#[cfg(test)]
mod whole;

pub use self::paper::{A4, CARTA, Paper};
pub use self::product::{Laid, NothingPrinted, Pile, Printed, Skipped, piece_to_pdf, to_pdf};
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

/// What the pattern calls a piece.
///
/// Asked here and not of the piece's own ink, because a piece that never
/// reached paper has no ink and still has to be named in the summary that says
/// why.
fn named(draft: &Draft, piece: PieceKey) -> String {
    draft
        .doc()
        .pieces
        .get(piece)
        .map_or_else(String::new, |held| held.name.clone())
}

/// The objects the file is made of, in the order they are written.
///
/// A catalogue, a page tree, the one font they all name, and then two objects
/// per sheet. Helvetica is one of the fourteen every reader carries, so nothing
/// has to be embedded and the file stays what it draws.
///
/// Each page states its own paper as its box, so printing at 100 % is the
/// identity and a viewer's fit-to-page is the only thing left that can lie
/// about the scale — which is why every sheet says in Spanish not to use it.
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
///
/// No clock reaches the bytes and no object id depends on anything but a
/// sheet's rank, so the file is a function of the document alone.
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
