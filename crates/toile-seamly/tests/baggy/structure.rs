use toile_seamly::{NodeKind, SplineLength};

use super::{BODY, PATTERN, body, evaluate, pattern};

#[test]
fn the_owner_pattern_reads_into_ten_blocks_of_one_piece_each() {
    let pattern = pattern();
    assert_eq!(pattern.blocks.len(), 10);
    assert!(pattern.blocks.iter().all(|block| block.pieces.len() == 1));
    assert_eq!(pattern.variables.len(), 18);
    let internal: usize = pattern.blocks.iter().map(|b| b.internal_paths.len()).sum();
    assert_eq!(internal, 25);
    let notches = pattern
        .blocks
        .iter()
        .flat_map(|b| &b.pieces)
        .flat_map(|p| &p.outline)
        .filter(|node| node.notch.is_some())
        .count();
    assert_eq!(notches, 2);
}

#[test]
fn every_path_node_resolves_to_an_evaluated_object_of_its_kind() {
    let pattern = pattern();
    let evaluation = evaluate(SplineLength::ArcLength);
    for block in &pattern.blocks {
        let outlines = block.pieces.iter().flat_map(|p| &p.outline);
        let internal = block.internal_paths.iter().flat_map(|p| &p.nodes);
        for node in outlines.chain(internal) {
            let evaluated = match node.kind {
                NodeKind::Point => evaluation.points.contains_key(&node.object),
                NodeKind::Spline => evaluation.splines.contains_key(&node.object),
                NodeKind::SplinePath => evaluation.paths.contains_key(&node.object),
                NodeKind::Arc => evaluation.arcs.contains_key(&node.object),
            };
            assert!(
                evaluated,
                "{}: node {:?} resolves to nothing",
                block.name, node
            );
        }
        for piece in &block.pieces {
            for path in &piece.internal_paths {
                assert!(block.internal_paths.iter().any(|p| p.id == *path));
            }
        }
    }
}

#[test]
fn every_modeling_copy_stands_for_an_object_of_its_own_block() {
    for block in &pattern().blocks {
        for original in block.copies.values() {
            assert!(block.objects.iter().any(|o| o.id == *original));
        }
    }
}

/// Nothing the file's `<personal>` block says survives the reading.
///
/// The fixture's block holds sentinels in place of the owner's details, so
/// a leak shows as a word no measurement file would otherwise contain.
#[test]
fn the_personal_block_of_the_owner_measurements_is_never_read() {
    let open = BODY.find("<personal>").expect("the file has one");
    let close = BODY.find("</personal>").expect("and closes it");
    let said: Vec<&str> = BODY[open..close]
        .split(['<', '>'])
        .step_by(2)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .collect();
    assert_eq!(
        said,
        [
            "SENTINEL-GIVEN-NAME",
            "SENTINEL-BIRTH-DATE",
            "SENTINEL-GENDER"
        ]
    );
    let read = format!("{:?}", body());
    assert!(!read.contains("SENTINEL"), "the personal block was read");
}

/// Every comment is found where it sits, and the note Seamly stamps on the
/// file is told apart from the author's.
#[test]
fn every_comment_is_placed_by_line_and_block_and_the_signature_is_told_apart() {
    let pattern = pattern();
    let comments = &pattern.comments;
    assert_eq!(comments.len(), PATTERN.matches("<!--").count());
    for comment in comments {
        let line = PATTERN
            .lines()
            .nth(comment.line as usize - 1)
            .expect("a line");
        assert!(line.contains("<!--"), "line {}", comment.line);
    }
    let signed: Vec<_> = comments.iter().filter(|c| c.signature).collect();
    assert_eq!(signed.len(), 1);
    assert_eq!((signed[0].line, signed[0].block.as_deref()), (3, None));
    let notes = |block: Option<&str>| {
        comments
            .iter()
            .filter(|c| !c.signature && c.block.as_deref() == block)
            .count()
    };
    assert_eq!(notes(None), 1);
    assert_eq!(notes(Some("Baggy Jeans Front [Muller]")), 2);
    assert_eq!(notes(Some("back")), 14);
    assert_eq!(notes(Some("chain cover strip")), 1);
}
