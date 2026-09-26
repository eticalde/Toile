use super::*;
use crate::{Elastic, InternalLine, LineKind, LineSpan, MannequinKey, PieceKey, PointKey, block};

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
