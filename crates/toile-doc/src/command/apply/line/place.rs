use super::super::curve::place;
use super::super::range::anchored;
use crate::{
    Doc, DocError, Identity, InternalLine, LineEdit, LineKey, LineVertex, PointKey, VertexEdit,
};

/// One place of a line on its way in, checked against the document.
///
/// A place on the contour answers to the very door a seam side answers to. A
/// place the document already carries only has to be a point that is still
/// there: a line may start at a corner, and nothing about a free point is
/// looked up. `brought` is the answer for the one citation whose point is not
/// there yet — a run that names one point twice takes the key back at the first
/// of the two places and cites it at the other, and this check runs before
/// either has been written. A place the drawing creates is checked by `fits`,
/// which is where the keys are.
pub(super) fn placed(doc: &Doc, vertex: &VertexEdit, brought: &[PointKey]) -> Result<(), DocError> {
    match vertex {
        VertexEdit::Contour(anchor) => anchored(doc, *anchor),
        VertexEdit::Free { .. } => Ok(()),
        VertexEdit::Cited(point) => match doc.points.get(*point) {
            Some(_) => Ok(()),
            None if brought.contains(point) => Ok(()),
            None => Err(DocError::stale(*point)),
        },
    }
}

/// The keys the edit's own places take back, in run order.
pub(super) fn brought(edit: &LineEdit) -> Vec<PointKey> {
    edit.vertices()
        .filter_map(|vertex| match vertex {
            VertexEdit::Free {
                identity: Identity::Restored(key),
                ..
            } => Some(*key),
            VertexEdit::Free { .. } | VertexEdit::Contour(_) | VertexEdit::Cited(_) => None,
        })
        .collect()
}

/// Puts one place of a line into the document and hands back the place the
/// line holds.
pub(super) fn seated(doc: &mut Doc, vertex: VertexEdit) -> Result<LineVertex, DocError> {
    match vertex {
        VertexEdit::Contour(anchor) => Ok(LineVertex::Contour(anchor)),
        VertexEdit::Free { identity, value } => Ok(LineVertex::free(place(doc, identity, value)?)),
        VertexEdit::Cited(point) => Ok(LineVertex::free(point)),
    }
}

/// The points a removal takes away with the line, in the order the run names
/// them and each of them once.
///
/// A point anything else in the document names outlives the line: another line,
/// a contour, a dart or an axis of symmetry goes on needing it, and taking it
/// away would break what that other thing draws. A run that names one point
/// twice is still the only thing naming it — which is how a run that closes is
/// drawn — so that point is the line's too and goes with it. Once: there is one
/// point to take away however many of the run's places sit on it.
pub(super) fn own_points(doc: &Doc, line: LineKey, held: &InternalLine) -> Vec<PointKey> {
    let mut own: Vec<PointKey> = Vec::new();
    for point in held.vertices().filter_map(LineVertex::point) {
        if !own.contains(&point) && !cited(doc, line, point) {
            own.push(point);
        }
    }
    own
}

/// One place of the line as the edit that would draw it again.
///
/// A place whose point the removal takes away carries the point back, key and
/// bindings and name; every other place cites the point it sits on. `back` is
/// what the places before this one carried, so a point the run names twice
/// comes back at the first of them and is cited at the rest: two places asking
/// for one key is the plan no arena can arrange, and either way it is the same
/// point.
pub(super) fn given_back(
    doc: &Doc,
    vertex: LineVertex,
    own: &[PointKey],
    back: &mut Vec<PointKey>,
) -> Result<VertexEdit, DocError> {
    match vertex {
        LineVertex::Contour(anchor) => Ok(VertexEdit::Contour(anchor)),
        LineVertex::Free { point } if own.contains(&point) && !back.contains(&point) => {
            let value = doc
                .points
                .get(point)
                .ok_or_else(|| DocError::stale(point))?
                .clone();
            back.push(point);
            Ok(VertexEdit::restored(point, value))
        }
        LineVertex::Free { point } => Ok(VertexEdit::Cited(point)),
    }
}

/// Whether anything but `line` still names `point`.
///
/// Every place in the document a point can be named from, because the question
/// is whether taking it away would leave something drawing with a key that
/// leads nowhere — which the file's own reader refuses, by design, on the way
/// back in.
fn cited(doc: &Doc, line: LineKey, point: PointKey) -> bool {
    !doc.pieces_citing(point).is_empty()
        || doc
            .lines
            .iter()
            .any(|(key, held)| key != line && held.cites(point))
        || doc
            .darts
            .iter()
            .any(|(_, held)| held.apex == point || held.legs.0 == point || held.legs.1 == point)
        || doc
            .symmetries
            .iter()
            .any(|(_, held)| held.axis.head.from == point || held.axis.tail.from == point)
}
