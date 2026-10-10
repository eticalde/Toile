use toile_seamly::{NodeKind, SplineLength};

use super::{BODY, PATTERN, body, evaluate, pattern};

/// One row of `CUT`: a piece's name, its letter, how many to cut, the seam
/// allowance in centimetres, whether the label says on the fold, and the
/// second line of its label.
type Cut = (
    &'static str,
    &'static str,
    u32,
    Option<f64>,
    bool,
    &'static str,
);

/// What the owner's file says about cutting out each of its ten pieces, read
/// off `<piece>` and its `<data>`.
///
/// `None` is a piece the file cuts on its own line, which it writes as
/// `seamAllowance="false"` and not as a width of nothing: the `width` beside
/// it is `0` on both of them. The first line of every label is the piece's
/// own name, which the test asserts rather than repeating here. The phrase is
/// never read for a number: `SOLAPA TRASERA` says two where the garment wants
/// four, and both of those are its author's.
const CUT: [Cut; 10] = [
    ("DELANTERO", "A", 2, Some(1.5), false, "cortar 2 espejadas"),
    (
        "TRASERO + CANESU",
        "B",
        2,
        Some(1.5),
        false,
        "cortar 2 - separar canesu",
    ),
    (
        "PRETINA",
        "C",
        1,
        Some(1.5),
        true,
        "cortar 1 al doblez c.b.",
    ),
    ("BOLSA PILL DELANTERA", "D", 2, Some(1.0), false, "cortar 2"),
    ("BOLSA PILL TRASERA", "E", 2, Some(1.0), false, "cortar 2"),
    (
        "TIRA PASACINTOS",
        "F",
        1,
        None,
        false,
        "cortar 1 - doblar en tercios",
    ),
    (
        "VISTA BRAGUETA",
        "G",
        2,
        Some(1.0),
        false,
        "ojales - cortar 2",
    ),
    (
        "PANEL BOTONES",
        "H",
        1,
        Some(1.0),
        false,
        "cortar 1 - doblar",
    ),
    ("TIRA CADENA", "I", 1, None, false, "al bies - repetir"),
    (
        "SOLAPA TRASERA",
        "J",
        2,
        Some(1.0),
        false,
        "cortar 2 + entretela",
    ),
];

/// Every piece's letter, count, allowance, fold and label, as the file writes
/// them and in file order; and not one of the ten writes a grain angle.
#[test]
fn the_ten_pieces_say_how_they_are_cut_out_exactly_as_the_owner_wrote_it() {
    let pattern = pattern();
    let pieces: Vec<_> = pattern.blocks.iter().flat_map(|b| &b.pieces).collect();
    assert_eq!(pieces.len(), CUT.len());
    for (piece, (name, letter, quantity, allowance, on_fold, phrase)) in pieces.iter().zip(CUT) {
        assert_eq!(piece.name, name);
        assert_eq!(piece.letter, letter, "{name}");
        assert_eq!(piece.quantity, quantity, "{name}");
        assert_eq!(piece.seam_allowance, allowance, "{name}");
        assert_eq!(piece.on_fold, on_fold, "{name}");
        assert_eq!(piece.labels, [name, phrase], "{name}");
        assert_eq!(piece.grain, None, "{name}");
    }
    let net: Vec<&str> = CUT
        .iter()
        .filter(|row| row.3.is_none())
        .map(|row| row.0)
        .collect();
    assert_eq!(net, ["TIRA PASACINTOS", "TIRA CADENA"]);
}

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
