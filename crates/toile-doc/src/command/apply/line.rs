mod place;

use place::{brought, given_back, own_points, placed, seated};

use super::curve::{install, puts_back, uninstall};
use crate::{
    Applied, ChangeClass, Command, Doc, DocError, Identity, InternalLine, LineEdit, LineKey,
    LineKind, LineSpan, Point, PointKey, SegmentEdit, SpanEdit, VertexEdit,
};

/// Draws a line on a piece that the pattern does not cut.
///
/// Nothing moves until the whole plan is known to fit. A place off the contour
/// and a span that bends both bring points that become points of the document,
/// and a line half drawn would leave a document no inverse describes.
pub(crate) fn add_line(
    doc: &mut Doc,
    identity: Identity<InternalLine>,
    edit: LineEdit,
) -> Result<Applied, DocError> {
    edit.check()?;
    // A line of free places names no piece anywhere else, and it is still cut
    // out with one.
    if doc.pieces.get(edit.piece).is_none() {
        return Err(DocError::stale(edit.piece));
    }
    let brought = brought(&edit);
    for vertex in edit.vertices() {
        placed(doc, vertex, &brought)?;
    }
    fits(doc, identity, &edit)?;
    let LineEdit {
        piece,
        kind,
        label,
        head,
        spans,
    } = edit;
    // In run order, each place before the handles of the span that reaches it:
    // the arena issues keys in the order it is asked for them, so this is the
    // order every drawing of this line repeats.
    let head = seated(doc, head)?;
    let mut drawn = Vec::with_capacity(spans.len());
    for span in spans {
        drawn.push(LineSpan {
            to: seated(doc, span.to)?,
            segment: install(doc, span.segment)?,
            samples: span.samples,
        });
    }
    let held = InternalLine {
        piece,
        kind,
        label,
        head,
        spans: drawn,
    };
    let key = match identity {
        Identity::New => doc.lines.insert(held),
        Identity::Restored(key) => {
            doc.lines.restore(key, held)?;
            key
        }
    };
    Ok(Applied {
        inverse: Command::RemoveLine { line: key },
        touched: Vec::new(),
        class: ChangeClass::Metadata,
    })
}

/// Takes an internal line off the piece it was drawn on.
///
/// The inverse carries the line back under its own key, with every point of its
/// own — the handles its curved spans hung on, and the places that sat on no
/// contour — under the keys and the bindings they had, so a gesture that rubs a
/// line out and an undo that draws it again leave the document it started from.
/// A point anything else still names stays where it is: it outlives the line,
/// so the inverse only cites it.
pub(crate) fn remove_line(doc: &mut Doc, line: LineKey) -> Result<Applied, DocError> {
    let held = doc
        .lines
        .get(line)
        .ok_or_else(|| DocError::stale(line))?
        .clone();
    let own = own_points(doc, line, &held);
    let mut back: Vec<PointKey> = Vec::new();
    let head = given_back(doc, held.head, &own, &mut back)?;
    let mut spans = Vec::with_capacity(held.spans.len());
    for span in &held.spans {
        spans.push(SpanEdit {
            to: given_back(doc, span.to, &own, &mut back)?,
            segment: puts_back(doc, span.segment)?,
            samples: span.samples,
        });
    }
    doc.lines.remove(line)?;
    for span in &held.spans {
        uninstall(doc, span.segment)?;
    }
    for point in own {
        doc.points.remove(point)?;
    }
    let edit = LineEdit {
        piece: held.piece,
        kind: held.kind,
        label: held.label,
        head,
        spans,
    };
    Ok(Applied {
        inverse: Command::AddLine {
            identity: Identity::Restored(line),
            line: Box::new(edit),
        },
        touched: Vec::new(),
        class: ChangeClass::Metadata,
    })
}

/// Writes what an internal line is for.
pub(crate) fn set_kind(doc: &mut Doc, line: LineKey, to: LineKind) -> Result<Applied, DocError> {
    let held = doc
        .lines
        .get_mut(line)
        .ok_or_else(|| DocError::stale(line))?;
    let from = std::mem::replace(&mut held.kind, to);
    Ok(Applied {
        inverse: Command::SetLineKind { line, to: from },
        touched: Vec::new(),
        class: ChangeClass::Metadata,
    })
}

/// Names an internal line, or takes its name away.
///
/// No name is refused for being taken: nothing is looked up by a line's name,
/// so two lines sharing one costs nothing, and seven belt loops keep the names
/// their author wrote.
pub(crate) fn label_line(
    doc: &mut Doc,
    line: LineKey,
    to: Option<String>,
) -> Result<Applied, DocError> {
    let held = doc
        .lines
        .get_mut(line)
        .ok_or_else(|| DocError::stale(line))?;
    let from = std::mem::replace(&mut held.label, to);
    Ok(Applied {
        inverse: Command::LabelLine { line, to: from },
        touched: Vec::new(),
        class: ChangeClass::Metadata,
    })
}

/// Checks every key the drawing claims, before anything moves.
///
/// The line's own key, each handle a restored span asks for and each place that
/// takes a key back have to name an open slot, and no two of them may want the
/// same one: two points landing on one key is the plan that cannot fit however
/// the arena is arranged.
fn fits(doc: &Doc, identity: Identity<InternalLine>, edit: &LineEdit) -> Result<(), DocError> {
    if let Identity::Restored(key) = identity
        && !doc.lines.is_vacant(key)
    {
        return Err(match doc.lines.get(key) {
            Some(_) => DocError::occupied(key),
            None => DocError::stale(key),
        });
    }
    let mut taken: Vec<PointKey> = Vec::new();
    for vertex in edit.vertices() {
        if let VertexEdit::Free { identity, .. } = vertex {
            claimed(doc, *identity, &mut taken)?;
        }
    }
    for span in &edit.spans {
        let SegmentEdit::Cubic(handles) = &span.segment else {
            continue;
        };
        for handle in [&handles.out, &handles.into] {
            claimed(doc, handle.identity, &mut taken)?;
        }
    }
    Ok(())
}

/// One key a point on its way in asks for, against the keys already claimed.
fn claimed(
    doc: &Doc,
    identity: Identity<Point>,
    taken: &mut Vec<PointKey>,
) -> Result<(), DocError> {
    let Identity::Restored(key) = identity else {
        return Ok(());
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
    Ok(())
}
