use super::FormatError;
use crate::{Arena, Doc, DocError, EdgeAnchor, EdgeRange, Key, LineVertex};

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
        live(&doc.points, symmetry.axis.0)?;
        live(&doc.points, symmetry.axis.1)?;
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
mod tests {
    use super::*;
    use crate::{
        Elastic, InternalLine, LineKind, LineSpan, MannequinKey, PieceKey, PointKey, block,
    };

    /// A hand-edited elastic is named rather than handed to the solver: a
    /// ratio of `1e9` throws a chain four hundred thousand kilometres long,
    /// a strength of `0` is a compliance no edge can be solved at, and one of
    /// `1e-300` is a compliance under which the edge is never solved.
    #[test]
    fn an_elastic_holding_a_stretch_to_numbers_no_elastic_holds_is_named() {
        let doc = block::trouser_front();
        let front = doc.piece_named(block::FRONT).expect("the block draws one");
        let named = |label| {
            doc.shows_label(front, label)
                .unwrap_or_else(|| panic!("the block names {label}"))
        };
        let at = EdgeRange::between(front, named("cintura_cf"), named("cadera_lat"));
        assert_eq!(elastics(&doc), Ok(()), "the block holds none");
        for (ratio, strength) in [
            (-5.0, 10.0),
            (1.0e9, 10.0),
            (0.0, 10.0),
            (0.85, 0.0),
            (0.85, 1.0e-300),
        ] {
            let mut edited = doc.clone();
            edited.elastics.insert(Elastic::new(at, ratio, strength));
            let error = elastics(&edited).expect_err("the numbers are not an elastic's");
            assert!(
                error.to_string().starts_with("an elastic in the pattern"),
                "{ratio} at {strength}: {error}"
            );
        }
        let mut held = doc.clone();
        held.elastics.insert(Elastic::new(at, 0.85, 10.0));
        assert_eq!(elastics(&held), Ok(()), "and a waistband passes");
    }

    /// A hand-edited line is named rather than drawn: a run through one place
    /// is a dot, an anchor on another piece's contour is cloth that is not
    /// there, and a fraction of `1.5` walks off the end of its tract.
    #[test]
    fn an_internal_line_no_piece_could_be_drawn_with_is_named() {
        let doc = block::trouser_front();
        let front = doc.piece_named(block::FRONT).expect("the block draws one");
        let node = doc
            .shows_label(front, "cintura_cf")
            .expect("the block names it");
        let head = LineVertex::Contour(EdgeAnchor::at_node(front, node));
        let drawn = |piece, head, spans| InternalLine {
            piece,
            kind: LineKind::Fold,
            label: None,
            head,
            spans,
        };
        let span = |to| LineSpan {
            to,
            segment: crate::Segment::Line,
            samples: 1,
        };
        let elsewhere = LineVertex::Contour(EdgeAnchor::at_node(PieceKey::new(9, 0), node));
        let past_the_end = LineVertex::Contour(EdgeAnchor {
            piece: front,
            from: node,
            t: 1.5,
        });
        assert_eq!(lines(&doc), Ok(()), "the block draws none");
        for bad in [
            drawn(front, head, Vec::new()),
            drawn(front, head, vec![span(elsewhere)]),
            drawn(front, head, vec![span(past_the_end)]),
        ] {
            let mut edited = doc.clone();
            edited.lines.insert(bad);
            let error = lines(&edited).expect_err("no piece is drawn with that");
            assert!(
                error.to_string().starts_with("the pattern draws"),
                "{error}"
            );
        }
        let mut held = doc.clone();
        held.lines.insert(drawn(front, head, vec![span(head)]));
        assert_eq!(lines(&held), Ok(()), "and a fold passes");
    }

    #[test]
    fn a_pattern_that_cites_only_what_it_carries_passes() {
        assert_eq!(references(&block::trousers()), Ok(()));
    }

    #[test]
    fn a_body_the_file_does_not_carry_is_named() {
        let mut doc = block::trouser_front();
        doc.resolve_with = MannequinKey::new(9, 0);
        let error = references(&doc).expect_err("the body is missing");
        assert_eq!(
            error.to_string(),
            "the pattern points at something the file does not carry: \
             `MeasureSet` has no entry 9.0"
        );
    }

    #[test]
    fn a_sampling_the_flattening_could_not_afford_is_named() {
        let mut doc = block::trouser_front();
        let front = doc.piece_named(block::FRONT).expect("the block draws one");
        doc.pieces.get_mut(front).expect("the key is live").contour[0].samples = u16::MAX;
        let error = samplings(&doc).expect_err("the count is past the ceiling");
        assert!(error.to_string().contains("asks for 65535"), "{error}");
        assert_eq!(samplings(&block::trouser_front()), Ok(()));
    }

    #[test]
    fn a_node_whose_point_is_missing_is_named() {
        let mut doc = block::trouser_front();
        let front = doc.piece_named(block::FRONT).expect("the block draws one");
        doc.pieces.get_mut(front).expect("the key is live").contour[2].point = PointKey::new(40, 0);
        let error = references(&doc).expect_err("the point is missing");
        assert!(error.to_string().contains("`Point` has no entry 40.0"));
    }
}
