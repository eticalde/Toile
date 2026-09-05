use super::ast::Op;
use super::lex::{self, Token, TokenKind};

/// Hundredths of a centimetre: what a nudged constant is written to.
///
/// A tenth of a millimetre is under the resolution of cloth and under the
/// width of a thread, so quantising there costs the pattern nothing and keeps
/// the file diffable through a whole drag.
const PLACES: f64 = 100.0;

/// Half of the last decimal written: a delta under this cannot show.
const INVISIBLE: f64 = 0.5 / PLACES;

/// A replacement in the source: which bytes go, and what takes their place.
struct Edit {
    at: usize,
    end: usize,
    text: String,
}

/// The source with `delta` centimetres absorbed into its adjustment term.
///
/// The term is rewritten by byte span, so every other byte of the source —
/// the spacing its author typed included — comes out identical. A source with
/// no such term gains one. The delta is never spread over a product: a
/// coordinate that reads `cadera / 4` keeps dividing by four.
///
/// The delta is first rounded to `step`, the snap in force; a step that is
/// not a positive number leaves it as it came.
pub fn rewrite(src: &str, delta: f64, step: f64) -> String {
    let delta = quantize(delta, step);
    if !delta.is_finite() || delta.abs() < INVISIBLE {
        return src.to_owned();
    }
    let tokens = lex::tokenize(src).expect("a formula's source tokenizes, since it parsed");
    if tokens.is_empty() {
        return src.to_owned();
    }
    let mut edits = Vec::new();
    absorb(&tokens, 0, tokens.len(), delta, &mut edits);
    splice(src, edits)
}

/// The delta rounded to the snap in force.
fn quantize(delta: f64, step: f64) -> f64 {
    if !step.is_finite() || step <= 0.0 {
        return delta;
    }
    (delta / step).round() * step
}

/// Writes the delta into the region `tokens[lo..hi)`, wherever it belongs.
fn absorb(tokens: &[Token], lo: usize, hi: usize, delta: f64, out: &mut Vec<Edit>) {
    if let Some((from, to)) = group(tokens, lo, hi) {
        absorb(tokens, from, to, delta, out);
        return;
    }
    if let Some((yes, no)) = branches(tokens, lo, hi) {
        // A conditional yields one number by choosing a branch, so an
        // adjustment that must hold whichever way it goes lands on both.
        absorb(tokens, yes.0, yes.1, delta, out);
        absorb(tokens, no.0, no.1, delta, out);
        return;
    }
    if let Some(edits) = adjustment(tokens, lo, hi, delta) {
        out.extend(edits);
        return;
    }
    let end = tokens[hi - 1].end;
    out.push(Edit {
        at: end,
        end,
        text: appended(delta),
    });
}

/// The rewrite of the region's own constant, when the region ends in one.
///
/// That is a region which is a bare number, or one whose last two tokens are
/// a binary `+` or `-` and a number. Anything else — a divisor, a call, a
/// name — is not an adjustment term and is left for the caller to append to.
fn adjustment(tokens: &[Token], lo: usize, hi: usize, delta: f64) -> Option<Vec<Edit>> {
    let last = &tokens[hi - 1];
    let &TokenKind::Number(value) = &last.kind else {
        return None;
    };
    if hi == lo + 1 {
        return Some(vec![replace(last, printed(value + delta))]);
    }
    let plus = match &tokens[hi - 2].kind {
        TokenKind::Op(Op::Add) => true,
        TokenKind::Op(Op::Sub) => false,
        _ => return None,
    };
    if hi - 2 == lo || !ends_operand(&tokens[hi - 3].kind) {
        return None;
    }
    // Rounded before its sign is read, so a term that lands on zero keeps
    // the operator its author typed instead of flipping on a rounding crumb.
    let moved = rounded(if plus { value } else { -value } + delta);
    let now_plus = moved >= 0.0;
    let mut edits = vec![replace(last, printed(moved.abs()))];
    // A term that changed side reads as `- 0.4`, never as `+ -0.4`.
    if now_plus != plus {
        let sign = if now_plus { "+" } else { "-" };
        edits.push(replace(&tokens[hi - 2], sign.to_owned()));
    }
    Some(edits)
}

/// Whether a token can end an operand, which is what makes the `+` after it
/// an addition rather than a sign.
fn ends_operand(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Number(_) | TokenKind::Name(_) | TokenKind::RParen
    )
}

/// The inside of the region, when the region is one parenthesised group.
fn group(tokens: &[Token], lo: usize, hi: usize) -> Option<(usize, usize)> {
    if tokens[lo].kind != TokenKind::LParen {
        return None;
    }
    let mut depth = 0i32;
    for (offset, token) in tokens[lo..hi].iter().enumerate() {
        match token.kind {
            TokenKind::LParen => depth += 1,
            TokenKind::RParen => {
                depth -= 1;
                if depth == 0 {
                    let close = lo + offset;
                    return (close == hi - 1 && close > lo + 1).then_some((lo + 1, close));
                }
            }
            _ => {}
        }
    }
    None
}

/// The two arms of the conditional the region is, if it is one.
fn branches(tokens: &[Token], lo: usize, hi: usize) -> Option<((usize, usize), (usize, usize))> {
    let mut depth = 0i32;
    let mut nested = 0u32;
    let mut question = None;
    for (offset, token) in tokens[lo..hi].iter().enumerate() {
        let index = lo + offset;
        match token.kind {
            TokenKind::LParen => depth += 1,
            TokenKind::RParen => depth -= 1,
            TokenKind::Question if depth == 0 => match question {
                None => question = Some(index),
                Some(_) => nested += 1,
            },
            TokenKind::Colon if depth == 0 => match (nested, question) {
                (0, Some(at)) => return Some(((at + 1, index), (index + 1, hi))),
                _ => nested -= 1,
            },
            _ => {}
        }
    }
    None
}

/// The term appended to a region that has no adjustment of its own.
fn appended(delta: f64) -> String {
    if delta < 0.0 {
        format!(" - {}", printed(-delta))
    } else {
        format!(" + {}", printed(delta))
    }
}

/// The value at the resolution a constant is written to.
fn rounded(value: f64) -> f64 {
    (value * PLACES).round() / PLACES
}

/// A constant as the document writes it.
fn printed(value: f64) -> String {
    let text = format!("{}", rounded(value));
    // A rounded-away negative prints as `-0`, which is not how zero is written.
    if text == "-0" { "0".to_owned() } else { text }
}

/// The edit that puts `text` where `token` stands.
fn replace(token: &Token, text: String) -> Edit {
    Edit {
        at: token.at,
        end: token.end,
        text,
    }
}

/// Applies the edits from the back, so the spans before them still hold.
fn splice(src: &str, mut edits: Vec<Edit>) -> String {
    edits.sort_by_key(|edit| std::cmp::Reverse(edit.at));
    let mut out = src.to_owned();
    for edit in edits {
        out.replace_range(edit.at..edit.end, &edit.text);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nudged at the tenth of a centimetre the app snaps to with no grid.
    fn nudged(src: &str, delta: f64) -> String {
        rewrite(src, delta, 0.1)
    }

    #[test]
    fn a_trailing_literal_absorbs_the_delta() {
        assert_eq!(nudged("cintura / 4 + 1", 0.6), "cintura / 4 + 1.6");
        assert_eq!(
            nudged("raya + ancho_bajo / 2 + 3", -1.0),
            "raya + ancho_bajo / 2 + 2"
        );
    }

    #[test]
    fn an_expression_without_one_gains_a_term() {
        assert_eq!(nudged("cadera / 4", 0.6), "cadera / 4 + 0.6");
        assert_eq!(nudged("min(1, 2)", 0.5), "min(1, 2) + 0.5");
        assert_eq!(nudged("-extension_tiro", 0.5), "-extension_tiro + 0.5");
        assert_eq!(nudged("2 ^ -1", 0.5), "2 ^ -1 + 0.5");
    }

    #[test]
    fn a_bare_number_just_changes() {
        assert_eq!(nudged("0", 0.6), "0.6");
        assert_eq!(nudged("22", -3.0), "19");
        assert_eq!(nudged("20.875", 0.1), "20.98");
    }

    #[test]
    fn a_negative_delta_reads_as_a_subtraction() {
        assert_eq!(nudged("cadera / 4", -0.4), "cadera / 4 - 0.4");
        assert_eq!(nudged("cintura / 4 + 1", -1.6), "cintura / 4 - 0.6");
        assert_eq!(nudged("cintura / 4 - 1", 1.5), "cintura / 4 + 0.5");
        assert_eq!(nudged("cintura / 4 - 1", -0.5), "cintura / 4 - 1.5");
    }

    #[test]
    fn the_divisor_is_never_the_term_that_moves() {
        assert_eq!(
            nudged("raya - ancho_bajo / 2", 0.6),
            "raya - ancho_bajo / 2 + 0.6"
        );
        assert_eq!(nudged("(a + b) * 2", 0.6), "(a + b) * 2 + 0.6");
    }

    #[test]
    fn the_delta_is_rounded_to_the_step() {
        assert_eq!(rewrite("1", 0.63, 0.5), "1.5");
        assert_eq!(rewrite("1", 0.63, 0.1), "1.6");
        assert_eq!(rewrite("1", 0.63, 0.0), "1.63");
        assert_eq!(rewrite("1", 0.638, f64::NAN), "1.64");
    }

    #[test]
    fn a_delta_too_small_to_show_leaves_the_source_alone() {
        assert_eq!(nudged("cintura / 4 + 1", 0.0), "cintura / 4 + 1");
        assert_eq!(rewrite("cintura / 4 + 1", 0.004, 0.0), "cintura / 4 + 1");
        assert_eq!(rewrite("cintura / 4 + 1", f64::NAN, 0.1), "cintura / 4 + 1");
    }
}
