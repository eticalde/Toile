use crate::piece::{check_allowance, check_quantity};
use crate::{Applied, ChangeClass, Command, Doc, DocError, PieceKey};

// None of the four names a piece as touched, for the reason a placement names
// none: a step through the history re-derives and re-drapes every piece it
// names, and nothing is derived from what a piece says about being cut out.
// The allowance does not offset the contour — the paper says the number — so
// the cloth that hangs on the body is the same cloth before the edit and
// after it.

/// Writes how far outside its drawn line a piece is cut.
pub(crate) fn set_seam_allowance(
    doc: &mut Doc,
    piece: PieceKey,
    to: Option<f64>,
) -> Result<Applied, DocError> {
    check_allowance(to)?;
    let held = doc
        .pieces
        .get_mut(piece)
        .ok_or_else(|| DocError::stale(piece))?;
    let from = std::mem::replace(&mut held.seam_allowance, to);
    Ok(Applied {
        inverse: Command::SetSeamAllowance { piece, to: from },
        touched: Vec::new(),
        class: ChangeClass::Metadata,
    })
}

/// Writes how many of a piece the garment takes.
pub(crate) fn set_quantity(doc: &mut Doc, piece: PieceKey, to: u32) -> Result<Applied, DocError> {
    check_quantity(to)?;
    let held = doc
        .pieces
        .get_mut(piece)
        .ok_or_else(|| DocError::stale(piece))?;
    let from = std::mem::replace(&mut held.quantity, to);
    Ok(Applied {
        inverse: Command::SetQuantity { piece, to: from },
        touched: Vec::new(),
        class: ChangeClass::Metadata,
    })
}

/// Writes the letter a piece's label shows, or takes it away.
///
/// Nothing here refuses a letter another piece already shows. A letter is what
/// its author wrote on the paper rather than a key, two pieces of one pattern
/// may honestly carry the same one, and a name is already the thing the
/// document keeps unique.
pub(crate) fn set_letter(
    doc: &mut Doc,
    piece: PieceKey,
    to: Option<String>,
) -> Result<Applied, DocError> {
    let held = doc
        .pieces
        .get_mut(piece)
        .ok_or_else(|| DocError::stale(piece))?;
    let from = std::mem::replace(&mut held.letter, to);
    Ok(Applied {
        inverse: Command::SetLetter { piece, to: from },
        touched: Vec::new(),
        class: ChangeClass::Metadata,
    })
}

/// Writes the lines of a piece's label, as its author wrote them.
///
/// The whole label at once, because that is the shape of the thing: a label is
/// read as the paragraph it is, and an edit that could move one line without
/// the others would need a second name for where a line sits.
pub(crate) fn set_labels(
    doc: &mut Doc,
    piece: PieceKey,
    to: Vec<String>,
) -> Result<Applied, DocError> {
    let held = doc
        .pieces
        .get_mut(piece)
        .ok_or_else(|| DocError::stale(piece))?;
    let from = std::mem::replace(&mut held.labels, to);
    Ok(Applied {
        inverse: Command::SetLabels { piece, to: from },
        touched: Vec::new(),
        class: ChangeClass::Metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block;

    fn front() -> (Doc, PieceKey) {
        let doc = block::trouser_front();
        let piece = doc.piece_named(block::FRONT).expect("the block draws one");
        (doc, piece)
    }

    #[test]
    fn an_allowance_that_takes_cloth_off_the_piece_is_refused() {
        let (mut doc, piece) = front();
        for width in [-0.1, f64::NAN, f64::INFINITY] {
            let refused = set_seam_allowance(&mut doc, piece, Some(width));
            assert_eq!(refused, Err(DocError::SeamAllowance), "{width}");
        }
        set_seam_allowance(&mut doc, piece, Some(0.0)).expect("a piece may be cut on its line");
        assert_eq!(set_quantity(&mut doc, piece, 0), Err(DocError::CutQuantity));
    }

    /// Every one of the four hands back the value it overwrote, which is what
    /// an undo plays to get the document's bytes back.
    #[test]
    fn each_edit_undoes_to_the_piece_that_was_there() {
        let (mut doc, piece) = front();
        let before = doc.to_canonical_json();
        let edits = [
            Command::SetSeamAllowance {
                piece,
                to: Some(1.5),
            },
            Command::SetQuantity { piece, to: 2 },
            Command::SetLetter {
                piece,
                to: Some("A".to_owned()),
            },
            Command::SetLabels {
                piece,
                to: vec!["DELANTERO".to_owned(), "cortar 2 espejadas".to_owned()],
            },
        ];
        let mut inverses = Vec::new();
        for edit in edits {
            inverses.push(edit.apply(&mut doc).expect("the front is live").inverse);
        }
        assert_ne!(doc.to_canonical_json(), before);
        for inverse in inverses.into_iter().rev() {
            inverse.apply(&mut doc).expect("every inverse applies");
        }
        assert_eq!(doc.to_canonical_json(), before);
    }
}
