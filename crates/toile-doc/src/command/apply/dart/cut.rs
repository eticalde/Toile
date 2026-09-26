use super::super::curve::place;
use super::super::topology::seat;
use super::{sew, slots, three_places};
use crate::dart::wedge_seat;
use crate::piece::samples_fit;
use crate::{
    Applied, ChangeClass, Command, ContourNode, Dart, DartKey, DartWedge, Doc, DocError, Identity,
    PieceKey, PointKey, SeamKey, WedgeNode,
};

/// Cuts a dart: the wedge out of the contour, and the seam that closes it.
///
/// Those two things and no third. The wedge leaves a simple polygon the mesher
/// already fills, and closing the dart is an ordinary seam between the two
/// legs, so nothing new reaches the solver. Nothing moves until the whole plan
/// is known to fit: the cut brings three points, a seam and the dart itself,
/// and half of it would leave a document no inverse describes.
///
/// A topology change and not a shape one: the contour gains three nodes, so the
/// mesh built from it cannot be warm-started and has to be built again.
///
/// The keys the dart names are the ones this edit issues, so they are read off
/// the cut rather than off the `Dart` the command carries — only the fold is
/// taken from there. The identity says which way round that goes: `New` cuts a
/// dart and hands out fresh keys, `Restored` is undo giving every one of them
/// back, the three nodes and the seam included, so that a redo writes the file
/// the first cut wrote.
pub(crate) fn add_dart(
    doc: &mut Doc,
    identity: Identity<Dart>,
    dart: Dart,
    wedge: DartWedge,
) -> Result<Applied, DocError> {
    let DartWedge {
        piece,
        after,
        nodes,
    } = wedge;
    let at = seat(doc, piece, after)?;
    // Nor a second wedge into the middle of the first: see `dart::over`.
    if crate::dart::splits(doc, piece, at) {
        return Err(DocError::InsideAWedge);
    }
    three_places([&nodes[0].value, &nodes[1].value, &nodes[2].value])?;
    for node in &nodes {
        if !samples_fit(node.segment.bends(), node.samples) {
            return Err(DocError::sampling(node.samples));
        }
    }
    fits(doc, identity, &nodes, dart.seam)?;

    let mut cut: Vec<ContourNode> = Vec::with_capacity(nodes.len());
    for node in &nodes {
        let point = place(doc, node.identity, node.value.clone())?;
        cut.push(node.node(point));
    }
    let apex = cut[1].point;
    let legs = (cut[0].point, cut[2].point);
    let seam = sew(doc, identity, dart.seam, piece, apex, legs)?;
    let held = doc
        .pieces
        .get_mut(piece)
        .ok_or_else(|| DocError::stale(piece))?;
    for (offset, node) in cut.into_iter().enumerate() {
        held.contour.insert(at + offset, node);
    }
    let made = Dart {
        apex,
        legs,
        seam,
        fold: dart.fold,
    };
    let key = match identity {
        Identity::New => doc.darts.insert(made),
        Identity::Restored(key) => {
            doc.darts.restore(key, made)?;
            key
        }
    };
    Ok(Applied {
        inverse: Command::RemoveDart { dart: key },
        touched: vec![piece],
        class: ChangeClass::Topology,
    })
}

/// Closes a dart back up: the wedge out of the contour, and its seam with it.
///
/// The contour comes back node for node, because the three nodes the cut put in
/// are the three it takes out and the tract that reached the wedge was never
/// touched — it simply runs to the node beyond again. The inverse carries the
/// whole cut back: the wedge's nodes under their own keys with the points they
/// had grown, the seam under its key, the dart under its own, and where in the
/// contour the wedge followed.
pub(crate) fn remove_dart(doc: &mut Doc, dart: DartKey) -> Result<Applied, DocError> {
    let held = *doc.darts.get(dart).ok_or_else(|| DocError::stale(dart))?;
    let sewn = *doc
        .seams
        .get(held.seam)
        .ok_or_else(|| DocError::stale(held.seam))?;
    // The seam is where the dart says which piece it is cut into: the record
    // names points, and a point on its own does not name the contour it is on.
    let piece = sewn.a.piece().ok_or(DocError::SplitSeamSide)?;
    let at = wedge_seat(
        doc.pieces
            .get(piece)
            .ok_or_else(|| DocError::stale(piece))?,
        [held.legs.0, held.apex, held.legs.1],
    )?;
    let (after, nodes) = taken(doc, piece, at)?;
    doc.darts.remove(dart)?;
    doc.seams.remove(held.seam)?;
    doc.pieces
        .get_mut(piece)
        .ok_or_else(|| DocError::stale(piece))?
        .contour
        .drain(at..at + nodes.len());
    for node in &nodes {
        if let Identity::Restored(key) = node.identity {
            doc.points.remove(key)?;
        }
    }
    Ok(Applied {
        inverse: Command::AddDart {
            identity: Identity::Restored(dart),
            dart: held,
            wedge: Box::new(DartWedge {
                piece,
                after,
                nodes,
            }),
        },
        touched: vec![piece],
        class: ChangeClass::Topology,
    })
}

/// Checks every key the cut claims, before anything moves.
///
/// A restored key has to name an open slot, and no two nodes of one wedge may
/// ask for the same one: two points landing on one key is the plan that cannot
/// fit however the arena is arranged.
fn fits(
    doc: &Doc,
    identity: Identity<Dart>,
    nodes: &[WedgeNode; 3],
    seam: SeamKey,
) -> Result<(), DocError> {
    let mut taken: Vec<PointKey> = Vec::new();
    for node in nodes {
        let Identity::Restored(key) = node.identity else {
            continue;
        };
        if taken.contains(&key) {
            return Err(DocError::occupied(key));
        }
        super::open(&doc.points, key)?;
        taken.push(key);
    }
    slots(doc, identity, seam)
}

/// The wedge as the edit that would cut it again: what it followed, and its
/// three nodes under their own keys with the points they hold now.
fn taken(
    doc: &Doc,
    piece: PieceKey,
    at: usize,
) -> Result<(Option<PointKey>, [WedgeNode; 3]), DocError> {
    let held = doc
        .pieces
        .get(piece)
        .ok_or_else(|| DocError::stale(piece))?;
    let after = at
        .checked_sub(1)
        .and_then(|before| held.contour.get(before))
        .map(|node| node.point);
    let node = |offset: usize| -> Result<WedgeNode, DocError> {
        let found = *held.contour.get(at + offset).ok_or(DocError::NoSuchNode)?;
        let value = doc
            .points
            .get(found.point)
            .ok_or_else(|| DocError::stale(found.point))?
            .clone();
        Ok(WedgeNode {
            identity: Identity::Restored(found.point),
            value,
            segment: found.segment,
            samples: found.samples,
        })
    };
    Ok((after, [node(0)?, node(1)?, node(2)?]))
}
