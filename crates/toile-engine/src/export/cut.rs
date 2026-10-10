use crate::draft::Piece;

/// How the sheet writes a piece's handle: the letter its author gave it, then
/// the name the document knows it by.
///
/// The letter leads because it is the shortest thing on the sheet, and a person
/// matching cut cloth against paper reads the short thing. The name follows
/// because the letter alone cannot be trusted to point at one piece: nothing
/// checks a letter for uniqueness, and the name is the one handle the document
/// does keep unique.
pub(super) fn handle(piece: &Piece) -> String {
    match &piece.letter {
        Some(letter) => format!("{letter} · «{}»", piece.name),
        None => format!("«{}»", piece.name),
    }
}

/// What the piece says about being cut out: the count with the allowance, then
/// the lines its author wrote.
///
/// The author's lines come last and come whole. None of them is read, dropped
/// or reordered — not even one that repeats the name the handle already shows,
/// because choosing which line to keep would mean reading the prose, and the
/// one thing a label is never read for here is what it says.
pub(super) fn says(piece: &Piece) -> Vec<String> {
    let mut out = vec![how(piece)];
    out.extend(piece.labels.iter().cloned());
    out
}

/// The line that answers the question asked at the cutting table.
///
/// The outline on the sheet is the line the piece is sewn on, and the line it
/// is cut on is that outline moved outward by the allowance — a move nothing
/// here makes. So the number is useless on its own: a cutter who takes the
/// printed outline for the cut line loses an allowance at every edge and two
/// across every seam, which is a garment. The sentence therefore says which
/// line the number is measured from.
///
/// A piece with no allowance says so in the same breath rather than leaving
/// the sentence off. Silence is what sent the owner back to his notes twice
/// while he cut these jeans, and two of his ten pieces are net.
fn how(piece: &Piece) -> String {
    let count = format!("Cortar {}", piece.quantity);
    match piece.seam_allowance {
        Some(margin) => format!(
            "{count} · margen {} cm por fuera del contorno",
            width(margin)
        ),
        None => format!("{count} · sin margen: corta por el contorno"),
    }
}

/// An allowance as the paper writes it, in centimetres.
///
/// Rounded to a hundredth, which is a tenth of a millimetre and finer than any
/// pair of scissors, so what the rounding drops is not a width a hand could
/// have cut to. Trailing zeros go with it: a pattern drawn in whole centimetres
/// says `1 cm`, which is what its author typed.
fn width(margin: f64) -> String {
    let rounded = format!("{margin:.2}");
    rounded
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draft::Winding;

    /// A piece carrying nothing but a name, which is what every piece drawn in
    /// Toile itself is.
    fn bare() -> Piece {
        Piece::polygon("Cuadro", [], Winding::Cw)
    }

    /// The owner's own notation reaches the paper: his `1.5` is not `1.50` and
    /// his `1` is not `1.0`.
    #[test]
    fn an_allowance_is_written_the_way_its_author_typed_it() {
        assert_eq!(width(1.5), "1.5");
        assert_eq!(width(1.0), "1");
        assert_eq!(width(10.0), "10");
        assert_eq!(width(0.75), "0.75");
        assert_eq!(width(0.0), "0");
    }

    /// The one thing silence would cost: a net piece says it is net.
    #[test]
    fn a_piece_with_no_allowance_says_so_rather_than_saying_nothing() {
        let said = says(&bare());
        assert_eq!(said, ["Cortar 1 · sin margen: corta por el contorno"]);
        assert!(!said[0].contains("margen 0"), "{said:?}");
    }

    /// And a piece with one says which line the number is measured from.
    #[test]
    fn a_piece_with_an_allowance_says_where_the_cut_line_is() {
        let mut piece = bare();
        piece.seam_allowance = Some(1.5);
        piece.quantity = 2;
        piece.letter = Some("A".to_owned());
        piece.labels = vec!["DELANTERO".to_owned(), "cortar 2 espejadas".to_owned()];
        assert_eq!(handle(&piece), "A · «Cuadro»");
        assert_eq!(
            says(&piece),
            [
                "Cortar 2 · margen 1.5 cm por fuera del contorno",
                "DELANTERO",
                "cortar 2 espejadas",
            ]
        );
    }

    /// A piece whose author gave it no letter is handled by its name alone, and
    /// not by an empty space where a letter would be.
    #[test]
    fn a_piece_with_no_letter_is_handled_by_its_name() {
        assert_eq!(handle(&bare()), "«Cuadro»");
    }
}
