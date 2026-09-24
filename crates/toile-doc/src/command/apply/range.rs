use crate::{Doc, DocError, EdgeAnchor, EdgeRange};

/// Both ends of a stretch of contour, checked against the document.
///
/// Every edit that writes a stretch asks here, and there is one set of rules
/// for all of them: a stretch that cites a dead point would sit in the document
/// waiting for the derive that cannot resolve it, so it is refused now, with
/// the key named.
pub(super) fn stretch(doc: &Doc, range: EdgeRange) -> Result<(), DocError> {
    anchored(doc, range.head)?;
    anchored(doc, range.tail)
}

/// One anchor, checked against the contour it claims to sit on.
///
/// The point has to be live, it has to be a node of the piece it names — a
/// handle is a point of the document too, but nothing anchors to a handle —
/// and the fraction has to be one that tract can answer for.
///
/// The fraction is checked here because this is the only door a stretch comes
/// through, and because the file is what pays for it: JSON has no spelling for
/// a NaN, so the writer spells one `null` and the product that saved cleanly
/// refuses to open again. A fraction past the end of its tract is refused
/// beside it, since a stretch that runs off the contour is not a stretch.
pub(super) fn anchored(doc: &Doc, anchor: EdgeAnchor) -> Result<(), DocError> {
    let held = doc
        .pieces
        .get(anchor.piece)
        .ok_or_else(|| DocError::stale(anchor.piece))?;
    if doc.points.get(anchor.from).is_none() {
        return Err(DocError::stale(anchor.from));
    }
    if held.node_index(anchor.from).is_none() {
        return Err(DocError::NoSuchNode);
    }
    if !anchor.is_valid() {
        return Err(DocError::AnchorFraction);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block;

    /// The stretch the block's own waist names, which every case here bends.
    fn waist(doc: &Doc) -> EdgeRange {
        let piece = doc.piece_named(block::FRONT).expect("the block draws one");
        let named = |label| {
            doc.shows_label(piece, label)
                .unwrap_or_else(|| panic!("the block names {label}"))
        };
        EdgeRange::between(piece, named("cintura_cf"), named("cadera_lat"))
    }

    /// A fraction the tract cannot answer for is refused at the one door every
    /// stretch comes through, so neither a seam nor an elastic can carry one.
    ///
    /// The NaN is the case that costs a product: it passes every other check,
    /// saves, and comes back as a `null` no reader takes for a number.
    #[test]
    fn a_fraction_no_tract_can_answer_for_is_refused_at_both_ends() {
        let doc = block::trouser_front();
        let good = waist(&doc);
        assert_eq!(stretch(&doc, good), Ok(()));
        for t in [f64::NAN, -0.5, 1.5, f64::INFINITY, f64::NEG_INFINITY] {
            let head = EdgeRange {
                head: EdgeAnchor { t, ..good.head },
                ..good
            };
            let tail = EdgeRange {
                tail: EdgeAnchor { t, ..good.tail },
                ..good
            };
            assert_eq!(stretch(&doc, head), Err(DocError::AnchorFraction), "{t}");
            assert_eq!(stretch(&doc, tail), Err(DocError::AnchorFraction), "{t}");
        }
    }

    /// And the fractions a tract does answer for are still taken: an anchor
    /// halfway along a tract is what a stretch that does not end on a node is
    /// made of.
    #[test]
    fn a_fraction_inside_the_tract_is_taken_at_either_end() {
        let doc = block::trouser_front();
        let good = waist(&doc);
        for t in [0.0, 0.5, 1.0] {
            let moved = EdgeRange {
                head: EdgeAnchor { t, ..good.head },
                tail: EdgeAnchor { t, ..good.tail },
            };
            assert_eq!(stretch(&doc, moved), Ok(()), "{t}");
        }
    }
}
