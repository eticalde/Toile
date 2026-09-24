use super::curve::{install, puts_back, uninstall};
use super::range::anchored;
use crate::{
    Applied, ChangeClass, Command, Doc, DocError, Identity, InternalLine, LineEdit, LineKey,
    LineKind, LineSpan, LineVertex, PointKey, SegmentEdit, SpanEdit,
};

/// Draws a line on a piece that the pattern does not cut.
///
/// Nothing moves until the whole plan is known to fit. A span that bends brings
/// two handles that become points of the document, and a line half drawn would
/// leave a document no inverse describes.
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
    for vertex in edit.vertices() {
        placed(doc, vertex)?;
    }
    fits(doc, identity, &edit)?;
    let LineEdit {
        piece,
        kind,
        label,
        head,
        spans,
    } = edit;
    let mut drawn = Vec::with_capacity(spans.len());
    for span in spans {
        drawn.push(LineSpan {
            to: span.to,
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
/// The inverse carries the line back under its own key, with every handle its
/// curved spans hung on under the key and the bindings it had, so a gesture
/// that rubs a line out and an undo that draws it again leave the document it
/// started from.
pub(crate) fn remove_line(doc: &mut Doc, line: LineKey) -> Result<Applied, DocError> {
    let held = doc
        .lines
        .get(line)
        .ok_or_else(|| DocError::stale(line))?
        .clone();
    let mut spans = Vec::with_capacity(held.spans.len());
    for span in &held.spans {
        spans.push(SpanEdit {
            to: span.to,
            segment: puts_back(doc, span.segment)?,
            samples: span.samples,
        });
    }
    doc.lines.remove(line)?;
    for span in &held.spans {
        uninstall(doc, span.segment)?;
    }
    let edit = LineEdit {
        piece: held.piece,
        kind: held.kind,
        label: held.label,
        head: held.head,
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

/// One place of a line, checked against the document.
///
/// A place on the contour answers to the very door a seam side answers to. A
/// free place only has to be a point the document still carries: a line may
/// start at a corner, and nothing about a free point is looked up.
fn placed(doc: &Doc, vertex: LineVertex) -> Result<(), DocError> {
    match vertex {
        LineVertex::Contour(anchor) => anchored(doc, anchor),
        LineVertex::Free { point } => match doc.points.get(point) {
            Some(_) => Ok(()),
            None => Err(DocError::stale(point)),
        },
    }
}

/// Checks every key the drawing claims, before anything moves.
///
/// The line's own key and each handle a restored span asks for have to name an
/// open slot, and no two of them may want the same one: two points landing on
/// one key is the plan that cannot fit however the arena is arranged.
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
    for span in &edit.spans {
        let SegmentEdit::Cubic(handles) = &span.segment else {
            continue;
        };
        for handle in [&handles.out, &handles.into] {
            let Identity::Restored(key) = handle.identity else {
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
    }
    Ok(())
}
