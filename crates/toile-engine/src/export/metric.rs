/// How wide the question mark is, in thousandths of the size it is set at.
///
/// Both sheets write a character the encoding cannot carry as one, so it is
/// the room anything else takes as well.
const QUESTION: u32 = 556;

/// How far above its own baseline a line of type may reach, as a share of the
/// size it is set at.
///
/// The font's own bounding box and not its cap height: an accented capital
/// reaches the first, and a layout that reserved the second would reserve a
/// height half the Spanish on these sheets rises past.
pub(super) const ABOVE: f64 = 0.931;

/// And how far below it, which is where a parenthesis and the tail of a `g`
/// go.
pub(super) const BELOW: f64 = 0.225;

/// How wide a line of type is, in the unit its size is given in.
pub(super) fn wide(size: f64, line: &str) -> f64 {
    size * line.chars().map(em).sum::<f64>()
}

/// The box a line of type takes, from the place its own left baseline is set
/// at, with the second axis running downward as the plane's does.
pub(super) fn box_of(size: f64, at: [f64; 2], line: &str) -> [f64; 4] {
    [
        at[0],
        at[1] - ABOVE * size,
        at[0] + wide(size, line),
        at[1] + BELOW * size,
    ]
}

/// How wide one character is, as a share of the size it is set at.
///
/// Measured and not estimated, because neither sheet embeds a font and both
/// name one whose widths are part of the format: the printed page asks for
/// Helvetica in `WinAnsiEncoding`, one of the fourteen every reader carries,
/// and the drawing asks for `sans-serif`, which is Arial or Helvetica
/// wherever it lands — and those two agree on every width here. An estimate
/// of so much per character cannot do this job: it is short by two
/// millimetres on `A · «DELANTERO»` and long by four on a line of lower case,
/// and what it is paying for is the promise that a line this module placed is
/// a line nothing else is written over.
fn em(character: char) -> f64 {
    let thousandths: u32 = match character {
        '\'' => 191,
        'i' | 'j' | 'l' | '\u{201a}' | '\u{2018}' | '\u{2019}' => 222,
        '|' | '¦' => 260,
        ' '
        | '!'
        | ','
        | '.'
        | '/'
        | ':'
        | ';'
        | 'I'
        | 'f'
        | 't'
        | '·'
        | '\u{a0}'
        | 'Ì'..='Ï'
        | 'ì'..='ï' => 278,
        '(' | ')' | '-' | '`' | 'r' | '¡' | '¨' | '\u{ad}' | '¯' | '´' | '¸' | '²' | '³' | '¹'
        | '\u{2c6}' | '\u{2dc}' | '\u{2039}' | '\u{203a}' | '\u{201e}' | '\u{201c}'
        | '\u{201d}' => 333,
        '{' | '}' => 334,
        '\u{2022}' => 350,
        '"' => 355,
        'º' => 365,
        'ª' => 370,
        '*' => 389,
        '°' => 400,
        '^' => 469,
        'c' | 'k' | 's' | 'v' | 'x' | 'y' | 'z' | 'J' | 'š' | 'ž' | 'ç' | 'ý' | 'ÿ' => 500,
        '¶' => 537,
        '0'..='9'
        | '#'
        | '$'
        | '_'
        | 'a'
        | 'b'
        | 'd'
        | 'e'
        | 'g'
        | 'h'
        | 'n'
        | 'o'
        | 'p'
        | 'q'
        | 'u'
        | 'L'
        | '¢'
        | '£'
        | '¤'
        | '¥'
        | '§'
        | 'µ'
        | '«'
        | '»'
        | 'ð'
        | 'þ'
        | 'à'..='å'
        | 'è'..='ë'
        | 'ñ'
        | 'ò'..='ö'
        | 'ù'..='ü'
        | '\u{20ac}'
        | '\u{192}'
        | '\u{2020}'
        | '\u{2021}'
        | '\u{2013}' => 556,
        '+' | '<' | '=' | '>' | '~' | '¬' | '±' | '×' | '÷' => 584,
        'F' | 'T' | 'Z' | 'Ž' | 'ß' | 'ø' | '¿' => 611,
        '&'
        | 'A'
        | 'B'
        | 'E'
        | 'K'
        | 'S'
        | 'V'
        | 'X'
        | 'Y'
        | 'Š'
        | 'Ÿ'
        | 'Ý'
        | 'Þ'
        | 'À'..='Å'
        | 'È'..='Ë' => 667,
        'C' | 'D' | 'H' | 'N' | 'R' | 'U' | 'w' | 'Ç' | 'Ð' | 'Ñ' | 'Ù'..='Ü' => 722,
        '©' | '®' => 737,
        'G' | 'O' | 'Q' | 'Ø' | 'Ò'..='Ö' => 778,
        'M' | 'm' => 833,
        '¼' | '½' | '¾' => 834,
        '%' | 'æ' => 889,
        'W' | 'œ' => 944,
        'Æ' | 'Œ' | '\u{2014}' | '\u{2026}' | '\u{2030}' | '\u{2122}' => 1000,
        '@' => 1015,
        _ => QUESTION,
    };
    f64::from(thousandths) / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The one number a reader can check against the font's own table by eye,
    /// and the one the whole layout leans on: the handle of the owner's front
    /// panel is 8.725 ems, which is 43.6 mm at the size a title is set.
    #[test]
    fn a_handle_measures_what_helvetica_sets_it_at() {
        assert!((wide(1.0, "A · «DELANTERO»") - 8.725).abs() < 1e-9);
        assert!((wide(5.0, "A · «DELANTERO»") - 43.625).abs() < 1e-9);
    }

    /// Both sheets write a character the encoding cannot carry as a question
    /// mark, so the room it takes is a question mark's.
    #[test]
    fn a_character_the_encoding_cannot_carry_takes_a_question_mark_of_room() {
        assert!((wide(3.0, "袖") - wide(3.0, "?")).abs() < 1e-9);
    }

    /// A box opens at the baseline it is told and reaches above and below it,
    /// because what a line of type overlaps is not its baseline.
    #[test]
    fn a_line_reaches_above_and_below_its_own_baseline() {
        let held = box_of(10.0, [4.0, 20.0], "m");
        assert!((held[1] - 10.69).abs() < 1e-9, "{held:?}");
        assert!((held[3] - 22.25).abs() < 1e-9, "{held:?}");
        assert!((held[2] - 12.33).abs() < 1e-9, "{held:?}");
    }
}
