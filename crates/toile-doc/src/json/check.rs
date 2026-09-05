use super::FormatError;
use crate::{Arena, Doc, DocError, EdgeAnchor, EdgeRange, Key};

/// Checks that no tract asks to be flattened at a count no tract can carry.
///
/// The count is a number a file chooses and the flattening is what every
/// resolve walks, twice over and pairwise, so a file left to name its own
/// count names how long opening it takes. It is refused here, by the piece
/// and the node, rather than clamped somewhere downstream where the document
/// would quietly stop being the one that was written.
pub(super) fn samplings(doc: &Doc) -> Result<(), FormatError> {
    for (_, piece) in doc.pieces.iter() {
        for node in &piece.contour {
            if !node.takes_samples(node.samples) {
                return Err(FormatError::Sampling(DocError::sampling(node.samples)));
            }
        }
    }
    Ok(())
}

/// Checks that every key the pattern cites names an entry the file carries.
///
/// A file can be edited by hand, and a key that leads nowhere would otherwise
/// only be found much later, by the drawing that cannot be drawn.
pub(super) fn references(doc: &Doc) -> Result<(), FormatError> {
    live(&doc.mannequins, doc.resolve_with)?;
    for (_, piece) in doc.pieces.iter() {
        for node in &piece.contour {
            live(&doc.points, node.point)?;
            if let Some((out, into)) = node.segment.handles() {
                live(&doc.points, out)?;
                live(&doc.points, into)?;
            }
        }
    }
    for (_, seam) in doc.seams.iter() {
        range(doc, seam.a)?;
        range(doc, seam.b)?;
    }
    for (_, notch) in doc.notches.iter() {
        anchor(doc, notch.at)?;
        if let Some(mate) = notch.mate {
            live(&doc.notches, mate)?;
        }
    }
    for (_, dart) in doc.darts.iter() {
        live(&doc.points, dart.apex)?;
        live(&doc.points, dart.legs.0)?;
        live(&doc.points, dart.legs.1)?;
        live(&doc.seams, dart.seam)?;
    }
    for (_, symmetry) in doc.symmetries.iter() {
        live(&doc.points, symmetry.axis.0)?;
        live(&doc.points, symmetry.axis.1)?;
    }
    for (_, pin) in doc.pins.iter() {
        live(&doc.pieces, pin.piece)?;
    }
    Ok(())
}

/// The stretch of contour both ends of a seam side name.
fn range(doc: &Doc, range: EdgeRange) -> Result<(), FormatError> {
    anchor(doc, range.head)?;
    anchor(doc, range.tail)
}

/// The piece and the node one place on a contour names.
fn anchor(doc: &Doc, anchor: EdgeAnchor) -> Result<(), FormatError> {
    live(&doc.pieces, anchor.piece)?;
    live(&doc.points, anchor.from)
}

/// The entry a key names, or the error that says which key names nothing.
fn live<T>(arena: &Arena<T>, key: Key<T>) -> Result<(), FormatError> {
    match arena.get(key) {
        Some(_) => Ok(()),
        None => Err(FormatError::Dangling(DocError::stale(key))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MannequinKey, PointKey, block};

    #[test]
    fn a_pattern_that_cites_only_what_it_carries_passes() {
        assert_eq!(references(&block::trouser_front()), Ok(()));
    }

    #[test]
    fn a_body_the_file_does_not_carry_is_named() {
        let mut doc = block::trouser_front();
        doc.resolve_with = MannequinKey::new(9, 0);
        let error = references(&doc).expect_err("the body is missing");
        assert_eq!(
            error.to_string(),
            "the pattern points at something the file does not carry: \
             `MeasureSet` has no entry 9.0"
        );
    }

    #[test]
    fn a_sampling_the_flattening_could_not_afford_is_named() {
        let mut doc = block::trouser_front();
        let front = doc.piece_named(block::FRONT).expect("the block draws one");
        doc.pieces.get_mut(front).expect("the key is live").contour[0].samples = u16::MAX;
        let error = samplings(&doc).expect_err("the count is past the ceiling");
        assert!(error.to_string().contains("asks for 65535"), "{error}");
        assert_eq!(samplings(&block::trouser_front()), Ok(()));
    }

    #[test]
    fn a_node_whose_point_is_missing_is_named() {
        let mut doc = block::trouser_front();
        let front = doc.piece_named(block::FRONT).expect("the block draws one");
        doc.pieces.get_mut(front).expect("the key is live").contour[2].point = PointKey::new(40, 0);
        let error = references(&doc).expect_err("the point is missing");
        assert!(error.to_string().contains("`Point` has no entry 40.0"));
    }
}
