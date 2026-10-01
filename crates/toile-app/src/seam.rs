use toile_engine::draft::{Draft, EdgeRange};
pub use toile_engine::draft::{Lengths, tolerance_cm};
use toile_engine::session::SeamFault;

/// Why a seam never reached the cloth: an end that is not on its own contour.
const LOOSE: &str = "un extremo no cae sobre un nodo de su pieza";
/// The same, for a side with no length between its two ends.
const PINCHED: &str = "los dos extremos caen en el mismo punto";

/// The two nodes a side runs between, under the names its piece shows.
pub fn name(draft: &Draft, range: &EdgeRange) -> Option<String> {
    let doc = draft.doc();
    let piece = range.piece()?;
    let head = doc.label_of(piece, range.head.from)?;
    let tail = doc.label_of(piece, range.tail.from)?;
    Some(format!("{head} → {tail}"))
}

/// Why the engine could not pair a seam onto the cloth, in the panels' words.
///
/// The fault is the engine's to find and the interface's to name, so the
/// wording is here and not in the error the engine raises.
pub fn why(fault: &SeamFault) -> &'static str {
    match fault {
        SeamFault::Unanchored => LOOSE,
        SeamFault::EmptyRange => PINCHED,
    }
}
