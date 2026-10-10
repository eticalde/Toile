/// Where the words a piece carries go, on paper and in a drawing alike.
mod block;
/// What a piece says about being cut out, in the language its cutter reads.
mod cut;
/// Everything a sheet draws of a piece apart from the line it is cut on.
mod drawn;
/// Whether any of the cloth falls on one box of the plane.
mod lands;
/// How much room one line of the two sheets' own type takes.
mod metric;
/// The pattern as sheets of paper a printer reproduces at true scale.
mod pdf;
mod svg;
/// How a document place, a document length and a document number reach a
/// sheet. Two formats draw the same pattern, so the millimetre, the two
/// decimals and the weight of a cut line have to be the same in both, or a
/// pattern printed and the same pattern drawn are not the same pattern.
mod units;

pub use drawn::Inked;
pub use pdf::{
    A4, CARTA, Laid, NothingPrinted, Paper, Pile, Printed, SheetError, Skipped, piece_to_pdf,
    to_pdf,
};
pub use svg::{Drawing, ExportError, to_svg};
