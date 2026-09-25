/// Everything a sheet draws of a piece apart from the line it is cut on.
mod drawn;
/// The pattern as sheets of paper a printer reproduces at true scale.
mod pdf;
mod svg;
/// How a document place, a document length and a document number reach a
/// sheet. Two formats draw the same pattern, so the millimetre, the two
/// decimals and the weight of a cut line have to be the same in both, or a
/// pattern printed and the same pattern drawn are not the same pattern.
mod units;

pub use drawn::Inked;
pub use pdf::{A4, CARTA, Paper, Printed, SheetError, to_pdf};
pub use svg::{ExportError, to_svg};
