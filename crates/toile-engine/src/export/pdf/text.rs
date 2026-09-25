use std::fmt::Write;

use super::super::units::number;
use super::paper::points;

/// One line of type on the page, anchored at its left baseline.
///
/// Told the size in millimetres, like every other length the sheet is laid out
/// in, because a type size a reader has to convert is a type size that will be
/// converted twice somewhere.
pub(super) fn show(out: &mut String, at: [f64; 2], size: f64, body: &str) {
    let _ = writeln!(
        out,
        "BT /F1 {} Tf {} {} Td {} Tj ET",
        number(points(size)),
        number(at[0]),
        number(at[1]),
        literal(body)
    );
}

/// A line of text as a `PDF` literal string, ready to hand to `Tj`.
///
/// Every byte comes out as printable `ASCII`: the three characters that would
/// close or confuse the string are backslashed, and everything above 126 is
/// written as an octal escape. The file stays readable in a terminal, and a
/// reader cannot mistake an accent for a delimiter.
pub(super) fn literal(text: &str) -> String {
    let mut out = String::from("(");
    for character in text.chars() {
        let code = win_ansi(character).unwrap_or(b'?');
        match code {
            b'\\' | b'(' | b')' => {
                out.push('\\');
                out.push(char::from(code));
            }
            0x20..=0x7e => out.push(char::from(code)),
            _ => {
                let _ = write!(out, "\\{code:03o}");
            }
        }
    }
    out.push(')');
    out
}

/// The code `WinAnsiEncoding` gives a character, or `None` when it gives none.
///
/// The encoding agrees with Unicode from U+0020 to U+00FF, so the accents,
/// the tildes and the angle quotes Spanish needs are the code point itself.
/// The dashes and curled quotes a word processor produces are not: Windows
/// put those where Unicode has control codes, and each one has to be named.
fn win_ansi(character: char) -> Option<u8> {
    match character {
        // A space that does not break is still a space to a printer, and the
        // encoding leaves its own 160 undefined.
        '\u{a0}' => Some(b' '),
        ' '..='~' | '\u{a1}'..='\u{ff}' => Some(character as u8),
        '\u{2013}' => Some(0x96),
        '\u{2014}' => Some(0x97),
        '\u{2018}' => Some(0x91),
        '\u{2019}' => Some(0x92),
        '\u{201c}' => Some(0x93),
        '\u{201d}' => Some(0x94),
        '\u{2022}' => Some(0x95),
        '\u{2026}' => Some(0x85),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The angle quotes and the accents a piece's name arrives with survive,
    /// as the codes the font was asked for and not as bytes a reader guesses.
    #[test]
    fn spanish_reaches_the_page_as_escapes_of_its_own_codes() {
        assert_eq!(literal("«Delantero»"), r"(\253Delantero\273)");
        assert_eq!(literal("Compruébalo"), r"(Compru\351balo)");
        assert_eq!(literal("año"), r"(a\361o)");
    }

    #[test]
    fn a_parenthesis_in_a_name_does_not_close_the_string() {
        assert_eq!(literal(r"a(b)\c"), r"(a\(b\)\\c)");
    }

    /// A character the font has no code for is written, not dropped: a name
    /// that lost something says so on the paper somebody cuts from.
    #[test]
    fn a_character_the_encoding_cannot_carry_is_still_written() {
        assert_eq!(literal("袖"), "(?)");
    }
}
