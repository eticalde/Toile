use super::FormatError;
use crate::{Arena, Doc, DocError, EdgeAnchor, EdgeRange, Key, LineVertex, PieceKey};

/// Checks that no tract asks to be flattened at a count no tract can carry.
///
/// The count is a number a file chooses and the flattening is what every
/// resolve walks, twice over and pairwise, so a file left to name its own
/// count names how long opening it takes. It is refused here, by the piece
/// and the node, rather than clamped somewhere downstream where the document
/// would quietly stop being the one that was written.
pub(super) fn samplings(doc: &Doc) -> Result<(), FormatError> {
    for (_, piece) in doc.pieces.iter() {
        for node in &piece.contour {
            if !node.takes_samples(node.samples) {
                return Err(FormatError::Sampling(DocError::sampling(node.samples)));
            }
        }
    }
    Ok(())
}

/// Checks that every elastic holds its stretch to numbers an elastic holds.
///
/// The same rule the edit answers to, asked again of the file, because the
/// file is the other way in: a hand-typed ratio of `-5` or `1e9` reaches the
/// solver as a rest length, and a strength of `0` or `1e-300` as a compliance
/// that leaves the stretch held by nothing, the cloth's own stiffness
/// included. Refused here rather than clamped, so the product that opens is
/// the product that was written or none at all.
pub(super) fn elastics(doc: &Doc) -> Result<(), FormatError> {
    for (_, elastic) in doc.elastics.iter() {
        elastic.check().map_err(FormatError::Elastic)?;
    }
    Ok(())
}

/// Checks that every hang is one a garment could be hung by.
///
/// The elastic's rule for its reason: a hand-typed station reaches the
/// placement as a height, and one no ring answers to names none.
pub(super) fn hangs(doc: &Doc) -> Result<(), FormatError> {
    for (_, hang) in doc.hangs.iter() {
        hang.check().map_err(FormatError::Hang)?;
    }
    Ok(())
}

/// Checks that every internal line is one a piece could be drawn with.
///
/// The same rule the edit answers to, asked again of the file, because the file
/// is the other way in: a hand-typed run through one place is not a line, an
/// anchor on another piece's contour is drawn on cloth that is not there, and a
/// fraction of `1.5` walks off the end of its tract. Refused here rather than
/// dropped, so the pattern that opens is the pattern that was written.
pub(super) fn lines(doc: &Doc) -> Result<(), FormatError> {
    for (_, line) in doc.lines.iter() {
        line.check().map_err(FormatError::InternalLine)?;
    }
    Ok(())
}

/// Checks that every axis is one a piece can be repeated across.
///
/// The same rule the edit answers to, asked again of the file, and the same
/// reason: a hand-typed axis across two pieces, or with both ends on one place,
/// names no line to mirror across, and two axes on one piece are a quarter
/// piece nothing here unfolds. A mirror is refused with them, since nothing
/// cuts the second piece yet and a reader that dropped it would cut one.
pub(super) fn symmetries(doc: &Doc) -> Result<(), FormatError> {
    let mut folded: Vec<PieceKey> = Vec::new();
    for (_, held) in doc.symmetries.iter() {
        held.check().map_err(FormatError::Symmetry)?;
        let piece = held
            .piece()
            .ok_or(FormatError::Symmetry(DocError::SplitSymmetry))?;
        if folded.contains(&piece) {
            return Err(FormatError::Symmetry(DocError::AlreadySymmetric));
        }
        folded.push(piece);
    }
    Ok(())
}

/// Checks every body's link to the library by the text it carries.
///
/// Its stem becomes a file name when a product opens, and a fingerprint of the
/// wrong shape would differ from every library file for good, so a link Toile
/// never wrote is refused here, before anything goes looking for it.
pub(super) fn origins(doc: &Doc) -> Result<(), FormatError> {
    for (_, set) in doc.mannequins.iter() {
        if let Some(origin) = &set.origin {
            origin.check().map_err(FormatError::Origin)?;
        }
    }
    Ok(())
}

/// Checks that every dart's record describes the contour it names.
///
/// The record says three things — three nodes standing together in one contour,
/// and a seam that sews leg to leg through the apex — and the file is the one
/// way in that can say them and be wrong. No edit can: the two that write a
/// dart write its seam themselves, and the only door out of a sewn seam refuses
/// a dart's. So this is where a hand, or a build that predates the wedge, is
/// held to the same rule, and refused rather than left to drape a dart that is
/// not shut and print a mouth across cloth nothing holds together.
pub(super) fn darts(doc: &Doc) -> Result<(), FormatError> {
    for (_, dart) in doc.darts.iter() {
        let sewn = doc
            .seams
            .get(dart.seam)
            .ok_or_else(|| FormatError::Dangling(DocError::stale(dart.seam)))?;
        let piece = sewn
            .a
            .piece()
            .ok_or(FormatError::Dart(DocError::SplitSeamSide))?;
        let held = doc
            .pieces
            .get(piece)
            .ok_or_else(|| FormatError::Dangling(DocError::stale(piece)))?;
        crate::dart::wedge_seat(held, [dart.legs.0, dart.apex, dart.legs.1])
            .map_err(FormatError::Dart)?;
        if !dart.is_shut_by(piece, sewn) {
            return Err(FormatError::Dart(DocError::DartNotShut));
        }
    }
    Ok(())
}

/// Checks that every key the pattern cites names an entry the file carries.
///
/// A file can be edited by hand, and a key that leads nowhere would otherwise
/// only be found much later, by the drawing that cannot be drawn.
pub(super) fn references(doc: &Doc) -> Result<(), FormatError> {
    live(&doc.mannequins, doc.resolve_with)?;
    for (_, piece) in doc.pieces.iter() {
        for node in &piece.contour {
            live(&doc.points, node.point)?;
            if let Some((out, into)) = node.segment.handles() {
                live(&doc.points, out)?;
                live(&doc.points, into)?;
            }
        }
    }
    for (_, seam) in doc.seams.iter() {
        range(doc, seam.a)?;
        range(doc, seam.b)?;
    }
    for (_, elastic) in doc.elastics.iter() {
        range(doc, elastic.at)?;
    }
    for (_, hang) in doc.hangs.iter() {
        range(doc, hang.at)?;
    }
    for (_, notch) in doc.notches.iter() {
        anchor(doc, notch.at)?;
        if let Some(mate) = notch.mate {
            live(&doc.notches, mate)?;
        }
    }
    for (_, drawn) in doc.lines.iter() {
        live(&doc.pieces, drawn.piece)?;
        for vertex in drawn.vertices() {
            match vertex {
                LineVertex::Contour(at) => anchor(doc, at)?,
                LineVertex::Free { point } => live(&doc.points, point)?,
            }
        }
        for handle in drawn.handles() {
            live(&doc.points, handle)?;
        }
    }
    for (_, dart) in doc.darts.iter() {
        live(&doc.points, dart.apex)?;
        live(&doc.points, dart.legs.0)?;
        live(&doc.points, dart.legs.1)?;
        live(&doc.seams, dart.seam)?;
    }
    for (_, symmetry) in doc.symmetries.iter() {
        range(doc, symmetry.axis)?;
    }
    for (_, pin) in doc.pins.iter() {
        live(&doc.pieces, pin.piece)?;
    }
    Ok(())
}

/// The stretch of contour both ends of a seam side name.
fn range(doc: &Doc, range: EdgeRange) -> Result<(), FormatError> {
    anchor(doc, range.head)?;
    anchor(doc, range.tail)
}

/// The piece and the node one place on a contour names.
fn anchor(doc: &Doc, anchor: EdgeAnchor) -> Result<(), FormatError> {
    live(&doc.pieces, anchor.piece)?;
    live(&doc.points, anchor.from)
}

/// The entry a key names, or the error that says which key names nothing.
fn live<T>(arena: &Arena<T>, key: Key<T>) -> Result<(), FormatError> {
    match arena.get(key) {
        Some(_) => Ok(()),
        None => Err(FormatError::Dangling(DocError::stale(key))),
    }
}

#[cfg(test)]
mod tests;
