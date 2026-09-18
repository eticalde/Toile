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
/// The point has to be live, and it has to be a node of the piece it names: a
/// handle is a point of the document too, but nothing anchors to a handle.
fn anchored(doc: &Doc, anchor: EdgeAnchor) -> Result<(), DocError> {
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
    Ok(())
}
