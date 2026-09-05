use super::Naming;
use super::curve::{install, place, puts_back, uninstall};
use crate::piece::samples_fit;
use crate::{
    Applied, ChangeClass, Command, ContourNode, Doc, DocError, Identity, Piece, PieceKey, Point,
    PointKey, SegmentEdit,
};

/// Puts a node into a contour, with the tract that leaves it.
///
/// Nothing moves until every key the edit asks for is known to fit: half an
/// insertion would leave the document in a state no inverse describes.
pub(crate) fn insert_node(
    doc: &mut Doc,
    piece: PieceKey,
    after: Option<PointKey>,
    identity: Identity<Point>,
    value: Point,
    segment: SegmentEdit,
    samples: u16,
) -> Result<Applied, DocError> {
    let seat = seat(doc, piece, after)?;
    if !samples_fit(segment.bends(), samples) {
        return Err(DocError::sampling(samples));
    }
    fits(doc, identity, &segment)?;
    let point = place(doc, identity, value)?;
    let made = install(doc, segment)?;
    doc.pieces
        .get_mut(piece)
        .ok_or_else(|| DocError::stale(piece))?
        .contour
        .insert(
            seat,
            ContourNode {
                point,
                segment: made,
                samples,
            },
        );
    Ok(Applied {
        inverse: Command::RemoveNode { piece, node: point },
        touched: vec![piece],
        class: ChangeClass::Topology,
    })
}

/// Takes a node out of a contour, and its point out of the document.
///
/// The tract that reached the node keeps the handles it had and now runs to
/// the node beyond: removing a node changes the line, which is what the person
/// asked for. What does not change is the identity of anything that stays, and
/// the inverse carries the whole node back — its key, its point, its handles
/// under their own keys and its sample count — so a seam that named it finds
/// it again after an undo.
pub(crate) fn remove_node(
    doc: &mut Doc,
    piece: PieceKey,
    node: PointKey,
) -> Result<Applied, DocError> {
    let held = doc
        .pieces
        .get(piece)
        .ok_or_else(|| DocError::stale(piece))?;
    let seat = held.node_index(node).ok_or(DocError::NoSuchNode)?;
    let found = *held.contour.get(seat).ok_or(DocError::NoSuchNode)?;
    let after = seat
        .checked_sub(1)
        .and_then(|before| held.contour.get(before))
        .map(|before| before.point);
    // A tract hanging on its own node would be removed twice over, and the
    // second removal would fail with the first one already done.
    if found.segment.cites(node) {
        return Err(DocError::occupied(node));
    }
    lone(doc, piece, seat, found)?;
    let value = doc
        .points
        .get(node)
        .ok_or_else(|| DocError::stale(node))?
        .clone();
    let segment = puts_back(doc, found.segment)?;
    uninstall(doc, found.segment)?;
    doc.points.remove(node)?;
    doc.pieces
        .get_mut(piece)
        .ok_or_else(|| DocError::stale(piece))?
        .contour
        .remove(seat);
    Ok(Applied {
        inverse: Command::InsertNode {
            piece,
            after,
            identity: Identity::Restored(node),
            value,
            segment,
            samples: found.samples,
        },
        touched: vec![piece],
        class: ChangeClass::Topology,
    })
}

/// Puts a piece on the table.
pub(crate) fn add_piece(
    doc: &mut Doc,
    identity: Identity<Piece>,
    piece: Piece,
    naming: Naming,
) -> Result<Applied, DocError> {
    if naming == Naming::Checked && doc.piece_named(&piece.name).is_some() {
        return Err(DocError::DuplicatePieceName(piece.name));
    }
    for point in cited(&piece) {
        if doc.points.get(point).is_none() {
            return Err(DocError::stale(point));
        }
    }
    let key = match identity {
        Identity::New => doc.pieces.insert(piece),
        Identity::Restored(key) => {
            doc.pieces.restore(key, piece)?;
            key
        }
    };
    Ok(Applied {
        inverse: Command::RemovePiece { piece: key },
        touched: vec![key],
        class: ChangeClass::Topology,
    })
}

/// Takes a piece off the table.
///
/// Its points stay in the document. They may be shared with another piece, and
/// even when they are not, leaving them is what makes the inverse a single
/// command that gives the piece back exactly as it was, every key intact.
pub(crate) fn remove_piece(doc: &mut Doc, piece: PieceKey) -> Result<Applied, DocError> {
    let held = doc.pieces.remove(piece)?;
    Ok(Applied {
        inverse: Command::AddPiece {
            identity: Identity::Restored(piece),
            piece: held,
        },
        touched: vec![piece],
        class: ChangeClass::Topology,
    })
}

/// Where in the contour a node inserted after `after` lands.
fn seat(doc: &Doc, piece: PieceKey, after: Option<PointKey>) -> Result<usize, DocError> {
    let held = doc
        .pieces
        .get(piece)
        .ok_or_else(|| DocError::stale(piece))?;
    match after {
        None => Ok(0),
        Some(point) => held
            .node_index(point)
            .map(|seat| seat + 1)
            .ok_or(DocError::NoSuchNode),
    }
}

/// Checks every key the insertion asks for, before anything moves.
///
/// A restored key has to name a slot that is open and no other part of the
/// same edit may want it: two points landing on one key is the plan that
/// cannot fit however the arena is arranged.
fn fits(doc: &Doc, identity: Identity<Point>, segment: &SegmentEdit) -> Result<(), DocError> {
    let mut taken: Vec<PointKey> = Vec::new();
    for identity in wanted(identity, segment) {
        let Identity::Restored(key) = identity else {
            continue;
        };
        if taken.contains(&key) {
            return Err(DocError::occupied(key));
        }
        if !doc.points.is_vacant(key) {
            return Err(match doc.points.get(key) {
                Some(_) => DocError::occupied(key),
                None => DocError::stale(key),
            });
        }
        taken.push(key);
    }
    Ok(())
}

/// Every identity the insertion claims: the node's, then its handles'.
fn wanted(identity: Identity<Point>, segment: &SegmentEdit) -> Vec<Identity<Point>> {
    let mut all = vec![identity];
    if let SegmentEdit::Cubic(handles) = segment {
        all.push(handles.out.identity);
        all.push(handles.into.identity);
    }
    all
}

/// Refuses a removal that would leave any live contour citing a dead key.
///
/// Points live in the document rather than inside a piece, so a point can be
/// named twice: by another piece, or by the owning piece itself at another
/// seat or as a handle. Taking the point away under either citation is not an
/// edit the inverse of this command could undo, so it is refused with the
/// dependent named rather than silently cascaded — the owning piece counts,
/// minus the one seat being removed.
fn lone(doc: &Doc, piece: PieceKey, seat: usize, node: ContourNode) -> Result<(), DocError> {
    for (key, other) in doc.pieces.iter() {
        let skip = (key == piece).then_some(seat);
        if still_cited(other, skip, node) {
            return Err(DocError::Shared(other.name.clone()));
        }
    }
    Ok(())
}

/// Whether a contour, minus the seat being skipped, names the node's point or
/// either of its handles.
fn still_cited(held: &Piece, skip: Option<usize>, node: ContourNode) -> bool {
    let (out, into) = node.segment.handles().unzip();
    let removed = [Some(node.point), out, into];
    held.contour
        .iter()
        .enumerate()
        .filter(|&(at, _)| Some(at) != skip)
        .any(|(_, entry)| {
            removed
                .iter()
                .flatten()
                .any(|&key| entry.point == key || entry.segment.cites(key))
        })
}

/// Every point a piece draws itself with, nodes and handles alike.
fn cited(piece: &Piece) -> impl Iterator<Item = PointKey> {
    piece.contour.iter().flat_map(|node| {
        let (out, into) = node.segment.handles().unzip();
        [Some(node.point), out, into].into_iter().flatten()
    })
}
