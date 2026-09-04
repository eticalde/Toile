use super::ast::{Cmp, Func, Op};
use super::syntax::{SyntaxError, SyntaxKind};

/// One token of a formula, with where it sits in the source.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// What the token is.
    pub kind: TokenKind,
    /// Byte offset of its first character.
    pub at: usize,
    /// Byte offset just past its last character.
    pub end: usize,
}

/// The kinds of token a formula is written with.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// A decimal literal.
    Number(f64),
    /// An identifier that is not a function name.
    Name(String),
    /// One of the four reserved function names.
    Func(Func),
    /// An arithmetic operator.
    Op(Op),
    /// A comparator.
    Cmp(Cmp),
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `,`
    Comma,
    /// `?`
    Question,
    /// `:`
    Colon,
}

/// Splits a formula's source text into tokens.
pub fn tokenize(src: &str) -> Result<Vec<Token>, SyntaxError> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let at = i;
        let kind = match bytes[i] {
            b'0'..=b'9' => number(src, &mut i)?,
            b'a'..=b'z' | b'_' => name(src, &mut i),
            b'<' | b'>' | b'=' | b'!' => comparator(src, &mut i)?,
            byte => {
                let Some(kind) = single(byte) else {
                    return Err(unexpected(src, at));
                };
                i += 1;
                kind
            }
        };
        out.push(Token { kind, at, end: i });
    }
    Ok(out)
}

/// The token a single character stands for, if it stands for one.
fn single(byte: u8) -> Option<TokenKind> {
    Some(match byte {
        b'+' => TokenKind::Op(Op::Add),
        b'-' => TokenKind::Op(Op::Sub),
        b'*' => TokenKind::Op(Op::Mul),
        b'/' => TokenKind::Op(Op::Div),
        b'^' => TokenKind::Op(Op::Pow),
        b'(' => TokenKind::LParen,
        b')' => TokenKind::RParen,
        b',' => TokenKind::Comma,
        b'?' => TokenKind::Question,
        b':' => TokenKind::Colon,
        _ => return None,
    })
}

/// Reads a run of digits and points as one decimal literal.
fn number(src: &str, i: &mut usize) -> Result<TokenKind, SyntaxError> {
    let bytes = src.as_bytes();
    let at = *i;
    while *i < bytes.len() && (bytes[*i].is_ascii_digit() || bytes[*i] == b'.') {
        *i += 1;
    }
    match src[at..*i].parse::<f64>() {
        Ok(value) if value.is_finite() => Ok(TokenKind::Number(value)),
        _ => Err(SyntaxError {
            at,
            kind: SyntaxKind::MalformedNumber,
        }),
    }
}

/// Reads an identifier, which may turn out to be a function name.
fn name(src: &str, i: &mut usize) -> TokenKind {
    let bytes = src.as_bytes();
    let at = *i;
    while *i < bytes.len()
        && (bytes[*i].is_ascii_lowercase() || bytes[*i].is_ascii_digit() || bytes[*i] == b'_')
    {
        *i += 1;
    }
    let text = &src[at..*i];
    Func::from_name(text).map_or_else(|| TokenKind::Name(text.to_owned()), TokenKind::Func)
}

/// Reads a comparator, which is one character or two.
fn comparator(src: &str, i: &mut usize) -> Result<TokenKind, SyntaxError> {
    let bytes = src.as_bytes();
    let at = *i;
    let paired = bytes.get(at + 1) == Some(&b'=');
    let cmp = match (bytes[at], paired) {
        (b'<', false) => Cmp::Lt,
        (b'<', true) => Cmp::Le,
        (b'>', false) => Cmp::Gt,
        (b'>', true) => Cmp::Ge,
        (b'=', true) => Cmp::Eq,
        (b'!', true) => Cmp::Ne,
        _ => return Err(unexpected(src, at)),
    };
    *i = at + if paired { 2 } else { 1 };
    Ok(TokenKind::Cmp(cmp))
}

fn unexpected(src: &str, at: usize) -> SyntaxError {
    let found = src[at..]
        .chars()
        .next()
        .expect("the scan only stops on a character boundary");
    SyntaxError {
        at,
        kind: SyntaxKind::UnexpectedChar(found),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        tokenize(src)
            .expect("the source tokenizes")
            .into_iter()
            .map(|token| token.kind)
            .collect()
    }

    #[test]
    fn an_empty_source_has_no_tokens() {
        assert!(tokenize("   ").expect("blanks tokenize").is_empty());
    }

    #[test]
    fn a_lone_point_is_not_a_number() {
        assert_eq!(
            tokenize(".5").unwrap_err().kind,
            SyntaxKind::UnexpectedChar('.')
        );
    }

    #[test]
    fn a_number_with_two_points_is_malformed() {
        let err = tokenize("1.2.3").unwrap_err();
        assert_eq!(err.at, 0);
        assert_eq!(err.kind, SyntaxKind::MalformedNumber);
    }

    #[test]
    fn a_token_keeps_the_span_it_came_from() {
        let tokens = tokenize("cadera / 4").expect("the source tokenizes");
        assert_eq!((tokens[0].at, tokens[0].end), (0, 6));
        assert_eq!((tokens[2].at, tokens[2].end), (9, 10));
    }

    #[test]
    fn a_two_character_comparator_is_one_token() {
        assert_eq!(
            kinds("<= >= == !="),
            vec![
                TokenKind::Cmp(Cmp::Le),
                TokenKind::Cmp(Cmp::Ge),
                TokenKind::Cmp(Cmp::Eq),
                TokenKind::Cmp(Cmp::Ne),
            ]
        );
    }

    #[test]
    fn a_lone_equals_sign_is_rejected() {
        assert_eq!(
            tokenize("a = b").unwrap_err().kind,
            SyntaxKind::UnexpectedChar('=')
        );
    }

    #[test]
    fn the_function_names_are_reserved() {
        assert_eq!(kinds("min"), vec![TokenKind::Func(Func::Min)]);
        assert_eq!(kinds("minimo"), vec![TokenKind::Name("minimo".to_owned())]);
    }

    #[test]
    fn a_capital_letter_is_not_part_of_a_name() {
        let err = tokenize("largoLateral").unwrap_err();
        assert_eq!(err.at, 5);
        assert_eq!(err.kind, SyntaxKind::UnexpectedChar('L'));
    }

    #[test]
    fn a_semicolon_is_not_a_separator() {
        let err = tokenize("min(1; 2)").unwrap_err();
        assert_eq!(err.at, 5);
        assert_eq!(err.kind, SyntaxKind::UnexpectedChar(';'));
    }

    #[test]
    fn a_non_ascii_character_reports_itself() {
        assert_eq!(
            tokenize("altura_caderá").unwrap_err().kind,
            SyntaxKind::UnexpectedChar('á')
        );
    }
}
