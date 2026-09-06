use crate::{
    Applied, ChangeClass, Command, Doc, DocError, EdgeAnchor, EdgeRange, Identity, PieceKey, Seam,
    SeamKey,
};

/// Sews two stretches of contour together.
///
/// Every anchor is checked before the seam lands: a seam that cites a dead
/// point would sit in the document waiting for the derive that cannot resolve
/// it, so it is refused here, with the key named.
pub(crate) fn add_seam(
    doc: &mut Doc,
    identity: Identity<Seam>,
    seam: Seam,
) -> Result<Applied, DocError> {
    side(doc, seam.a)?;
    side(doc, seam.b)?;
    let touched = touched(&seam);
    let key = match identity {
        Identity::New => doc.seams.insert(seam),
        Identity::Restored(key) => {
            doc.seams.restore(key, seam)?;
            key
        }
    };
    Ok(Applied {
        inverse: Command::RemoveSeam { seam: key },
        touched,
        class: ChangeClass::Topology,
    })
}

/// Unpicks a seam.
///
/// The inverse carries the seam back under its own key, so undoing the unpick
/// leaves anything that named the seam naming it still.
pub(crate) fn remove_seam(doc: &mut Doc, seam: SeamKey) -> Result<Applied, DocError> {
    let held = doc.seams.remove(seam)?;
    let touched = touched(&held);
    Ok(Applied {
        inverse: Command::AddSeam {
            identity: Identity::Restored(seam),
            seam: held,
        },
        touched,
        class: ChangeClass::Topology,
    })
}

/// One side of a seam, checked end to end.
fn side(doc: &Doc, range: EdgeRange) -> Result<(), DocError> {
    if range.piece().is_none() {
        return Err(DocError::SplitSeamSide);
    }
    anchored(doc, range.head)?;
    anchored(doc, range.tail)
}

/// One anchor, checked against the contour it claims to sit on.
///
/// The point has to be live, and it has to be a node of the piece it names: a
/// handle is a point of the document too, but nothing sews to a handle.
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

/// The pieces the seam sews, in key order.
fn touched(seam: &Seam) -> Vec<PieceKey> {
    let mut pieces = vec![seam.a.head.piece, seam.b.head.piece];
    pieces.sort_unstable();
    pieces.dedup();
    pieces
}
