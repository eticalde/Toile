use crate::{
    Applied, ChangeClass, Command, ContourNode, Doc, DocError, Handle, Handles, Identity, PieceKey,
    Point, PointKey, SAMPLES, Segment, SegmentEdit,
};

/// Changes what runs from a node to the next one, handles and all.
///
/// A cubic's handles are points of the document, so this is the edit that
/// creates them and the edit that takes them away. The inverse carries their
/// keys, which is why straightening a curve and taking it back gives the same
/// two points, with the bindings and names they had grown.
pub(crate) fn set_segment(
    doc: &mut Doc,
    piece: PieceKey,
    node: PointKey,
    to: SegmentEdit,
) -> Result<Applied, DocError> {
    let (index, held) = tract(doc, piece, node)?;
    let was = held.segment;
    sampled(held, &to)?;
    let undo = puts_back(doc, was)?;
    checked(doc, was, &to)?;
    let made = swap(doc, was, to)?;
    doc.pieces
        .get_mut(piece)
        .ok_or_else(|| DocError::stale(piece))?
        .contour
        .get_mut(index)
        .ok_or(DocError::NoSuchNode)?
        .segment = made;
    Ok(Applied {
        inverse: Command::SetSegment {
            piece,
            node,
            to: undo,
        },
        touched: vec![piece],
        class: ChangeClass::Topology,
    })
}

/// Changes how finely one tract is flattened.
///
/// The count is refused rather than clamped when it is not one the tract can
/// carry: a clamp would leave the document holding a number nobody wrote, and
/// the flattening this sizes is what every resolve walks twice over.
pub(crate) fn set_samples(
    doc: &mut Doc,
    piece: PieceKey,
    node: PointKey,
    to: u16,
) -> Result<Applied, DocError> {
    let (index, was) = tract(doc, piece, node)?;
    if !was.takes_samples(to) {
        return Err(DocError::sampling(to));
    }
    let held = doc
        .pieces
        .get_mut(piece)
        .ok_or_else(|| DocError::stale(piece))?
        .contour
        .get_mut(index)
        .ok_or(DocError::NoSuchNode)?;
    let from = std::mem::replace(&mut held.samples, to);
    Ok(Applied {
        inverse: Command::SetSamples {
            piece,
            node,
            to: from,
        },
        touched: vec![piece],
        class: ChangeClass::Topology,
    })
}

/// Where `node` sits in the contour of `piece`, and the node itself.
fn tract(doc: &Doc, piece: PieceKey, node: PointKey) -> Result<(usize, ContourNode), DocError> {
    let held = doc
        .pieces
        .get(piece)
        .ok_or_else(|| DocError::stale(piece))?;
    let index = held.node_index(node).ok_or(DocError::NoSuchNode)?;
    let found = *held.contour.get(index).ok_or(DocError::NoSuchNode)?;
    Ok((index, found))
}

/// Checks that the tract's sampling is one the segment it becomes can carry.
///
/// Bending is two edits, and the count comes first: a cubic flattened at one
/// sample is its own chord, so a tract may not take handles until it is
/// sampled finely enough to show them. The order is also what keeps undo
/// exact — the count goes back after the tract has straightened again, when
/// one is a count the tract can hold.
fn sampled(node: ContourNode, to: &SegmentEdit) -> Result<(), DocError> {
    if to.bends() && !(SAMPLES.0..=SAMPLES.1).contains(&node.samples) {
        return Err(DocError::sampling(node.samples));
    }
    Ok(())
}

/// The edit that puts `was` back, its handles and their bindings included.
pub(super) fn puts_back(doc: &Doc, was: Segment) -> Result<SegmentEdit, DocError> {
    let Some((out, into)) = was.handles() else {
        return Ok(SegmentEdit::Line);
    };
    Ok(SegmentEdit::curve(
        restored(doc, out)?,
        restored(doc, into)?,
    ))
}

/// A live handle as the edit that would give its key back.
fn restored(doc: &Doc, key: PointKey) -> Result<Handle, DocError> {
    let value = doc
        .points
        .get(key)
        .ok_or_else(|| DocError::stale(key))?
        .clone();
    Ok(Handle::restored(key, value))
}

/// Checks every key the edit needs while the old handles are still in place.
///
/// Half an applied edit would leave the document in a state no inverse
/// describes, so nothing moves until the whole plan is known to fit: the keys
/// the old handles hold are about to be free, and any other key the edit asks
/// for has to name an open slot already. Two handles asking for one key is the
/// one plan that cannot fit however the arena is arranged.
fn checked(doc: &Doc, was: Segment, to: &SegmentEdit) -> Result<(), DocError> {
    if let Some((out, into)) = was.handles()
        && out == into
    {
        return Err(DocError::occupied(out));
    }
    let SegmentEdit::Cubic(handles) = to else {
        return Ok(());
    };
    if handles.out.identity == handles.into.identity
        && let Identity::Restored(key) = handles.out.identity
    {
        return Err(DocError::occupied(key));
    }
    for handle in [&handles.out, &handles.into] {
        let Identity::Restored(key) = handle.identity else {
            continue;
        };
        if !was.cites(key) && !doc.points.is_vacant(key) {
            return Err(match doc.points.get(key) {
                Some(_) => DocError::occupied(key),
                None => DocError::stale(key),
            });
        }
    }
    Ok(())
}

/// Takes the handles of `was` out of the document and puts those of `to` in.
fn swap(doc: &mut Doc, was: Segment, to: SegmentEdit) -> Result<Segment, DocError> {
    uninstall(doc, was)?;
    install(doc, to)
}

/// Takes the handles a tract hangs on out of the document.
///
/// # Errors
/// `DocError::StaleKey` for a handle the document has already lost, which a
/// live tract cannot have.
pub(super) fn uninstall(doc: &mut Doc, was: Segment) -> Result<(), DocError> {
    if let Some((out, into)) = was.handles() {
        doc.points.remove(out)?;
        doc.points.remove(into)?;
    }
    Ok(())
}

/// Puts the handles an edit brings into the document and hands back the tract.
///
/// # Errors
/// `DocError::Occupied` for a key another point still holds, and
/// `DocError::StaleKey` for a key no slot answers to.
pub(super) fn install(doc: &mut Doc, to: SegmentEdit) -> Result<Segment, DocError> {
    match to {
        SegmentEdit::Line => Ok(Segment::Line),
        SegmentEdit::Cubic(handles) => {
            let Handles { out, into } = *handles;
            Ok(Segment::Cubic {
                out: place(doc, out.identity, out.value)?,
                into: place(doc, into.identity, into.value)?,
            })
        }
    }
}

/// Puts one point into the document under the key it asked for.
pub(super) fn place(
    doc: &mut Doc,
    identity: Identity<Point>,
    value: Point,
) -> Result<PointKey, DocError> {
    match identity {
        Identity::New => Ok(doc.points.insert(value)),
        Identity::Restored(key) => {
            doc.points.restore(key, value)?;
            Ok(key)
        }
    }
}
