use crate::{Dart, DartKey, Doc, EdgeAnchor, EdgeRange, LineVertex, PointKey};

/// Whether anything but the dart itself is drawn on the three nodes of its
/// wedge.
///
/// Closing a cut dart back up takes its three points out of the document, and
/// `json::check` refuses a file that names a point the file does not carry.
/// Measured before this answer existed: a mark put on a wedge's own leg
/// outlives the wedge, and the product saves and never opens again. So the
/// removal asks first, and so does the panel that offers it — one question, so
/// that what a person is offered and what the loader will accept cannot come
/// apart.
///
/// The dart's own seam is what shuts the wedge and comes off with it, so it
/// does not count. Neither do the wedge's own three seats in its own contour:
/// those are the wedge.
pub fn wedge_is_shared(doc: &Doc, dart: DartKey) -> bool {
    let Some(&held) = doc.darts.get(dart) else {
        return false;
    };
    let nodes = [held.legs.0, held.apex, held.legs.1];
    let on = |anchor: EdgeAnchor| nodes.contains(&anchor.from);
    let along = |at: EdgeRange| on(at.head) || on(at.tail);
    doc.seams
        .iter()
        .any(|(key, sewn)| key != held.seam && (along(sewn.a) || along(sewn.b)))
        || doc.elastics.iter().any(|(_, stretch)| along(stretch.at))
        || doc.hangs.iter().any(|(_, hung)| along(hung.at))
        || doc.notches.iter().any(|(_, notch)| on(notch.at))
        || doc.symmetries.iter().any(|(_, axis)| along(axis.axis))
        || doc.lines.iter().any(|(_, line)| {
            line.vertices().filter_map(LineVertex::anchor).any(on)
                || line.handles().any(|handle| nodes.contains(&handle))
        })
        || redrawn(doc, held, nodes)
}

/// Whether a contour draws one of the wedge's nodes anywhere but at the wedge.
///
/// A point lives in the document rather than inside a piece, so two pieces can
/// draw one corner and a tract can hang on a point as a handle. Only the three
/// seats the wedge itself occupies are skipped, which is why the seat is looked
/// up rather than the piece merely matched.
fn redrawn(doc: &Doc, dart: Dart, nodes: [PointKey; 3]) -> bool {
    let wedge = doc
        .seams
        .get(dart.seam)
        .and_then(|sewn| sewn.a.piece())
        .and_then(|piece| Some((piece, doc.pieces.get(piece)?)))
        .and_then(|(piece, held)| Some((piece, super::wedge_seat(held, nodes).ok()?)));
    doc.pieces.iter().any(|(key, held)| {
        held.contour.iter().enumerate().any(|(seat, node)| {
            let own = wedge
                .is_some_and(|(piece, at)| key == piece && (at..at + nodes.len()).contains(&seat));
            !own && nodes
                .iter()
                .any(|&point| node.point == point || node.segment.cites(point))
        })
    })
}
