use super::lex::{Token, TokenKind};
use super::syntax::{SyntaxError, SyntaxKind};

const MAX_DEPTH: u32 = 64;

/// Where the parser stands in a formula: which token, at what nesting.
pub struct Cursor<'t> {
    tokens: &'t [Token],
    next: usize,
    end: usize,
    depth: u32,
}

impl<'t> Cursor<'t> {
    /// A cursor on the first of `tokens`; `end` is the length of their source.
    pub fn new(tokens: &'t [Token], end: usize) -> Cursor<'t> {
        Cursor {
            tokens,
            next: 0,
            end,
            depth: 0,
        }
    }

    /// The token under the cursor.
    pub fn peek(&self) -> Option<&'t TokenKind> {
        self.tokens.get(self.next).map(|token| &token.kind)
    }

    /// Byte offset of the token under the cursor, or of the end of the source.
    pub fn at(&self) -> usize {
        self.tokens
            .get(self.next)
            .map_or(self.end, |token| token.at)
    }

    /// Steps past the token under the cursor.
    pub fn bump(&mut self) {
        self.next += 1;
    }

    /// Steps past the token under the cursor if it is the one expected.
    pub fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.peek() == Some(kind) {
            self.bump();
            return true;
        }
        false
    }

    /// Whether every token has been read.
    pub fn done(&self) -> bool {
        self.next >= self.tokens.len()
    }

    /// A fault of `kind` at the cursor.
    pub fn error(&self, kind: SyntaxKind) -> SyntaxError {
        SyntaxError::new(self.at(), kind)
    }

    /// Enters one level of nesting, or refuses to.
    ///
    /// The limit is deeper than any pattern formula needs and shallow enough
    /// that the recursive descent above it cannot overflow the stack.
    pub fn deepen(&mut self) -> Result<(), SyntaxError> {
        if self.depth == MAX_DEPTH {
            return Err(self.error(SyntaxKind::TooDeep));
        }
        self.depth += 1;
        Ok(())
    }

    /// Leaves one level of nesting.
    pub fn shallow(&mut self) {
        self.depth -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::super::lex::tokenize;
    use super::*;

    #[test]
    fn an_empty_stream_is_done_and_points_at_the_end() {
        let cursor = Cursor::new(&[], 7);
        assert!(cursor.done());
        assert_eq!(cursor.at(), 7);
        assert_eq!(cursor.peek(), None);
    }

    #[test]
    fn eating_the_wrong_token_leaves_the_cursor_where_it_was() {
        let tokens = tokenize("1 + 2").expect("the source tokenizes");
        let mut cursor = Cursor::new(&tokens, 5);
        assert!(!cursor.eat(&TokenKind::LParen));
        assert_eq!(cursor.at(), 0);
        assert!(cursor.eat(&TokenKind::Number(1.0)));
        assert_eq!(cursor.at(), 2);
    }

    #[test]
    fn nesting_past_the_limit_is_refused() {
        let mut cursor = Cursor::new(&[], 0);
        for _ in 0..MAX_DEPTH {
            cursor.deepen().expect("the limit is not reached yet");
        }
        assert_eq!(cursor.deepen().unwrap_err().kind, SyntaxKind::TooDeep);
        cursor.shallow();
        cursor.deepen().expect("leaving a level makes room again");
    }

    #[test]
    fn the_error_offset_is_the_token_under_the_cursor() {
        let tokens = tokenize("1 + 2").expect("the source tokenizes");
        let mut cursor = Cursor::new(&tokens, 5);
        cursor.bump();
        assert_eq!(cursor.error(SyntaxKind::TrailingInput).at, 2);
    }
}
