use super::*;
use crate::draft::{Doc, MeasureSet, Piece, Point, Winding, block};

/// A ten by twenty centimetre rectangle, whose every millimetre is known
/// without resolving anything.
fn rectangle() -> Draft {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
    let corners = [[0.0, 0.0], [10.0, 0.0], [10.0, 20.0], [0.0, 20.0]];
    let points: Vec<_> = corners
        .into_iter()
        .map(|[x, y]| doc.points.insert(Point::at(x, y)))
        .collect();
    doc.pieces
        .insert(Piece::polygon("Cuadro", points, Winding::Cw));
    Draft::from_doc(doc).expect("a rectangle resolves")
}

#[test]
fn the_sheet_is_declared_in_millimetres_at_true_scale() {
    let written = to_svg(&rectangle()).expect("a rectangle draws").text;
    assert!(
        written.contains(
            "width=\"120.00mm\" height=\"220.00mm\" viewBox=\"-10.00 -10.00 120.00 220.00\""
        ),
        "{written}"
    );
}

#[test]
fn a_contour_is_one_closed_path_in_millimetres() {
    let written = to_svg(&rectangle()).expect("a rectangle draws").text;
    assert!(
        written.contains("d=\"M 0.00 0.00 L 100.00 0.00 L 100.00 200.00 L 0.00 200.00 Z\""),
        "{written}"
    );
}

#[test]
fn svg_millimetres_match_the_resolved_contour() {
    let draft = Draft::from_doc(block::trouser_front()).expect("the block resolves");
    let piece = draft
        .doc()
        .piece_named(block::FRONT)
        .expect("the block draws one piece");
    let written = to_svg(&draft).expect("the block draws").text;
    for &(_, [x, y]) in draft.points_cm(piece).iter().take(3) {
        let [x, y] = units::millimetres([x, y]);
        let vertex = format!("{} {}", number(x), number(y));
        assert!(written.contains(&vertex), "{vertex} missing from {written}");
    }
}

/// What a ruler laid on the drawing reads, which is the one number the
/// whole of true scale is for.
#[test]
fn the_side_seam_measures_what_the_pattern_says_it_measures() {
    let draft = Draft::from_doc(block::trouser_front()).expect("the block resolves");
    let piece = draft
        .doc()
        .piece_named(block::FRONT)
        .expect("the block draws one piece");
    let drawn = points_of(&to_svg(&draft).expect("the block draws").text);
    // The waist opens the hip curve, the only bend before the hem, so the
    // hem sits past its samples plus the hip and the knee.
    let hip = draft
        .doc()
        .pieces
        .get(piece)
        .expect("the key is live")
        .contour[1]
        .samples;
    let hem = 1 + usize::from(hip) + 2;
    let side: f64 = drawn[1..=hem]
        .windows(2)
        .map(|step| {
            let (from, to) = (step[0], step[1]);
            (to[0] - from[0]).hypot(to[1] - from[1])
        })
        .sum();
    assert!((side - 1046.0).abs() < 0.5, "{side} mm");
}

/// The vertices of the first path of a drawing, in the order it draws
/// them, which is how a program that reads SVG would measure it.
fn points_of(drawing: &str) -> Vec<[f64; 2]> {
    let opened = drawing
        .split_once("d=\"")
        .expect("the drawing has a path")
        .1;
    let data = opened.split_once('"').expect("the path closes").0;
    let numbers: Vec<f64> = data
        .split_whitespace()
        .filter_map(|word| word.parse().ok())
        .collect();
    numbers
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| [pair[0], pair[1]])
        .collect()
}

#[test]
fn a_piece_carries_its_name_and_the_names_of_its_nodes() {
    let draft = Draft::from_doc(block::trouser_front()).expect("the block resolves");
    let written = to_svg(&draft).expect("the block draws").text;
    assert!(written.contains("<title>Delantero</title>"), "{written}");
    assert!(written.contains(">cintura_lat</text>"), "{written}");
}

#[test]
fn a_document_that_draws_nothing_is_not_a_drawing() {
    let doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
    let draft = Draft::from_doc(doc).expect("an empty document resolves");
    assert_eq!(to_svg(&draft), Err(ExportError::Empty));
}

/// Two pieces a centimetre apart, each named and labelled, so that what one
/// says has somewhere to be wrong.
fn beside() -> Draft {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
    for (name, from) in [("DELANTERO", 0.0), ("VISTA BRAGUETA", 13.0)] {
        let wide = if from > 0.0 { 5.0 } else { 12.0 };
        let corners = [
            [from, 0.0],
            [from + wide, 0.0],
            [from + wide, 17.0],
            [from, 17.0],
        ];
        let points: Vec<_> = corners
            .into_iter()
            .map(|[x, y]| doc.points.insert(Point::at(x, y)))
            .collect();
        let mut piece = Piece::polygon(name, points, Winding::Cw);
        piece.letter = Some(name[..1].to_owned());
        piece.quantity = 2;
        piece.seam_allowance = Some(1.0);
        doc.pieces.insert(piece);
    }
    Draft::from_doc(doc).expect("two rectangles resolve")
}

/// The drawing lays the words out by the same rule the paper does: a piece
/// five centimetres wide breaks its own sentences to five centimetres, and
/// none of them starts inside the piece beside it.
///
/// A drawing is one page, so nothing here is clipped — which is exactly why
/// the old answer, the words hung over the piece's top left corner, put them
/// on whatever was above and to the left.
#[test]
fn a_piece_in_a_drawing_keeps_its_words_inside_its_own_box() {
    let drawing = to_svg(&beside()).expect("both pieces draw").text;
    let said = typeset(&drawing);
    // A node's own name is not moved for anything, in either format, so what
    // is read here is the blocks: every line of them that is not one of the
    // automatic `P` names a bare rectangle's corners carry.
    let mut lines = 0;
    for (across, body) in said.iter().filter(|(_, body)| !body.starts_with('P')) {
        // Each block is inside its own piece's box, which on these two pieces
        // is 0 to 120 and 130 to 180 millimetres across.
        assert!(
            *across <= 120.0 || *across >= 130.0,
            "«{body}» opens between the two pieces, at {across} mm"
        );
        lines += 1;
    }
    // Two lines for the wide piece, and four for the narrow one: its handle
    // and its cutting instruction are both wider than it is.
    assert_eq!(lines, 6, "{said:?}");
    let narrow = said
        .iter()
        .filter(|(across, body)| *across >= 130.0 && !body.starts_with('P'))
        .count();
    assert_eq!(narrow, 4, "the narrow piece did not break its lines");
    // And both pieces still say what they say, in the words their author
    // wrote: broken, and whole.
    let whole: Vec<&str> = said.iter().map(|(_, body)| body.as_str()).collect();
    let whole = whole.join(" ");
    for want in [
        "Cortar 2 · margen 1 cm por fuera del contorno",
        "V · «VISTA BRAGUETA»",
    ] {
        assert!(whole.contains(want), "«{want}» is nowhere on the drawing");
    }
}

/// Every line of type on the drawing: where it opens across the sheet, and
/// what it says in the Spanish the file spells.
fn typeset(drawing: &str) -> Vec<(f64, String)> {
    drawing
        .lines()
        .filter(|line| line.contains("<text "))
        .filter_map(|line| {
            let across = line
                .split_once("x=\"")
                .and_then(|(_, rest)| rest.split_once('"'))
                .and_then(|(held, _)| held.parse().ok())?;
            let (body, _) = line.split_once('>')?.1.split_once("</text>")?;
            Some((across, body.replace("&#171;", "«").replace("&#187;", "»")))
        })
        .collect()
}

/// A piece whose words could only be written inside the piece beside it draws
/// without them, and says so where a viewer shows it.
///
/// A drawing has no legend to put a count in, so the one place left is the
/// piece's own title. Silence is the answer this whole decision exists to
/// refuse: a `Cortar 2` nobody can attribute is worse than a missing one.
#[test]
fn a_piece_with_nowhere_to_put_its_words_says_that_in_its_title() {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
    for (name, from, wide) in [("ANCHA", 0.2, 40.0), ("ESTRECHA", 0.0, 0.1)] {
        let corners = [
            [from, 0.0],
            [from + wide, 0.0],
            [from + wide, 30.0],
            [from, 30.0],
        ];
        let points: Vec<_> = corners
            .into_iter()
            .map(|[x, y]| doc.points.insert(Point::at(x, y)))
            .collect();
        doc.pieces.insert(Piece::polygon(name, points, Winding::Cw));
    }
    let draft = Draft::from_doc(doc).expect("two rectangles resolve");
    let drawing = to_svg(&draft).expect("the pair draws").text;
    assert!(
        drawing.contains("<title>ESTRECHA · sin sitio para su rótulo junto a la pieza</title>"),
        "{drawing}"
    );
    assert!(drawing.contains("<title>ANCHA</title>"), "{drawing}");
}
