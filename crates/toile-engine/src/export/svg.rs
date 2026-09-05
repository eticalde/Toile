mod mark;

use std::fmt::Write;

use crate::draft::{Draft, PieceKey, PointKey};

/// Millimetres in a centimetre. The document counts in the first and a sheet
/// of paper in the second, and this is the only place the two meet.
const MM_PER_CM: f64 = 10.0;

/// Blank paper left around the pattern, in millimetres.
const MARGIN: f64 = 10.0;

/// The weight of a cut line, in millimetres.
const CUT: f64 = 0.3;

/// What stops a pattern from being written as a drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ExportError {
    /// Nothing in the document resolves to a contour a sheet could carry.
    #[error("the pattern draws no piece that resolves to a contour")]
    Empty,
}

/// The document as an SVG drawing at true scale.
///
/// One user unit is one millimetre and the sheet declares itself in
/// millimetres, so the drawing measures on a ruler what the pattern says it
/// measures: the side seam of the base block is 104.5 cm in any program that
/// reads SVG. Each piece is one group — its cut line, its grain line and the
/// names it carries — so a piece can be moved or hidden on its own.
///
/// # Errors
/// `ExportError::Empty` when no piece of the document resolves to a contour.
pub fn to_svg(draft: &Draft) -> Result<String, ExportError> {
    let drawn: Vec<PieceKey> = draft
        .doc()
        .piece_keys()
        .into_iter()
        .filter(|&piece| draft.points_cm(piece).len() >= 3)
        .collect();
    let sheet = sheet(draft, &drawn).ok_or(ExportError::Empty)?;
    let mut out = String::new();
    header(&mut out, sheet);
    for piece in drawn {
        group(&mut out, draft, piece);
    }
    out.push_str("</svg>\n");
    Ok(out)
}

/// The sheet the pieces fit on: its corner and its size, in millimetres.
///
/// `None` when there is nothing on it, which is the one thing a drawing cannot
/// be made of.
fn sheet(draft: &Draft, pieces: &[PieceKey]) -> Option<[f64; 4]> {
    let mut low = [f64::INFINITY; 2];
    let mut high = [f64::NEG_INFINITY; 2];
    for &piece in pieces {
        for at in millimetres(draft, piece) {
            for axis in 0..2 {
                low[axis] = low[axis].min(at[axis]);
                high[axis] = high[axis].max(at[axis]);
            }
        }
    }
    if low[0] > high[0] {
        return None;
    }
    Some([
        low[0] - MARGIN,
        low[1] - MARGIN,
        high[0] - low[0] + 2.0 * MARGIN,
        high[1] - low[1] + 2.0 * MARGIN,
    ])
}

/// The opening of the file, where the true scale is declared.
fn header(out: &mut String, sheet: [f64; 4]) {
    let [x, y, w, h] = sheet.map(mm);
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = writeln!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}mm\" height=\"{h}mm\" \
         viewBox=\"{x} {y} {w} {h}\">"
    );
}

/// One piece: its cut line, its grain line, and the names it carries.
fn group(out: &mut String, draft: &Draft, piece: PieceKey) {
    let Some(held) = draft.doc().pieces.get(piece) else {
        return;
    };
    let outline = millimetres(draft, piece);
    let _ = writeln!(out, "  <g>");
    let _ = writeln!(out, "    <title>{}</title>", escape(&held.name));
    contour(out, &outline);
    mark::grain(out, &outline, held.grain.radians());
    mark::names(out, draft, piece, &outline, &held.name);
    let _ = writeln!(out, "  </g>");
}

/// The cut line: one closed path, in contour order.
fn contour(out: &mut String, outline: &[[f64; 2]]) {
    let mut path = String::new();
    for (rank, &[x, y]) in outline.iter().enumerate() {
        let verb = if rank == 0 { 'M' } else { 'L' };
        let _ = write!(path, "{verb} {} {} ", mm(x), mm(y));
    }
    path.push('Z');
    let _ = writeln!(
        out,
        "    <path d=\"{path}\" fill=\"none\" stroke=\"#000000\" stroke-width=\"{}\"/>",
        mm(CUT)
    );
}

/// A piece's contour in millimetres, in contour order.
fn millimetres(draft: &Draft, piece: PieceKey) -> Vec<[f64; 2]> {
    draft
        .points_cm(piece)
        .iter()
        .map(|&(_, [x, y])| [x * MM_PER_CM, y * MM_PER_CM])
        .collect()
}

/// The nodes of a piece, in contour order, so a name can be put beside one.
fn keys(draft: &Draft, piece: PieceKey) -> Vec<PointKey> {
    draft.points_cm(piece).iter().map(|&(key, _)| key).collect()
}

/// A millimetre as the drawing writes it: two decimals, and never a negative
/// zero, so the same pattern always writes the same bytes.
fn mm(value: f64) -> String {
    let text = format!("{value:.2}");
    if let Some(digits) = text.strip_prefix('-')
        && digits.bytes().all(|byte| byte == b'0' || byte == b'.')
    {
        return digits.to_owned();
    }
    text
}

/// Text as XML takes it.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
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
        let written = to_svg(&rectangle()).expect("a rectangle draws");
        assert!(
            written.contains(
                "width=\"120.00mm\" height=\"220.00mm\" viewBox=\"-10.00 -10.00 120.00 220.00\""
            ),
            "{written}"
        );
    }

    #[test]
    fn a_contour_is_one_closed_path_in_millimetres() {
        let written = to_svg(&rectangle()).expect("a rectangle draws");
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
        let written = to_svg(&draft).expect("the block draws");
        for &(_, [x, y]) in draft.points_cm(piece).iter().take(3) {
            let vertex = format!("{} {}", mm(x * MM_PER_CM), mm(y * MM_PER_CM));
            assert!(written.contains(&vertex), "{vertex} missing from {written}");
        }
    }

    /// What a ruler laid on the drawing reads, which is the one number the
    /// whole of true scale is for.
    #[test]
    fn the_side_seam_measures_what_the_pattern_says_it_measures() {
        let draft = Draft::from_doc(block::trouser_front()).expect("the block resolves");
        let drawn = points_of(&to_svg(&draft).expect("the block draws"));
        let side: f64 = drawn[1..=4]
            .windows(2)
            .map(|step| {
                let (from, to) = (step[0], step[1]);
                (to[0] - from[0]).hypot(to[1] - from[1])
            })
            .sum();
        assert!((side - 1044.8).abs() < 0.5, "{side} mm");
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
        let written = to_svg(&draft).expect("the block draws");
        assert!(written.contains("<title>Delantero</title>"), "{written}");
        assert!(written.contains(">cintura_lat</text>"), "{written}");
    }

    #[test]
    fn a_document_that_draws_nothing_is_not_a_drawing() {
        let doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
        let draft = Draft::from_doc(doc).expect("an empty document resolves");
        assert_eq!(to_svg(&draft), Err(ExportError::Empty));
    }

    #[test]
    fn a_negative_zero_is_written_as_zero() {
        assert_eq!(mm(-0.0), "0.00");
        assert_eq!(mm(-0.001), "0.00");
        assert_eq!(mm(-0.02), "-0.02");
    }

    #[test]
    fn a_name_that_carries_markup_is_escaped() {
        assert_eq!(escape("A & <B>"), "A &amp; &lt;B&gt;");
    }
}
