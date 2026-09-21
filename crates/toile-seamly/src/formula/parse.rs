use super::{Expr, FormulaError, Op};

/// Parses `source` with the usual precedence, every operator left
/// associative, so a formula evaluates in the order a Seamly reader expects.
pub(super) fn parse(source: &str) -> Result<Expr, FormulaError> {
    let mut parser = Parser { source, at: 0 };
    let expr = parser.sum()?;
    match parser.peek() {
        None => Ok(expr),
        Some(c) => Err(parser.unexpected(c)),
    }
}

struct Parser<'a> {
    source: &'a str,
    at: usize,
}

impl Parser<'_> {
    /// The next character after any whitespace, which it consumes.
    fn peek(&mut self) -> Option<char> {
        let rest = &self.source[self.at..];
        let trimmed = rest.trim_start();
        self.at += rest.len() - trimmed.len();
        trimmed.chars().next()
    }

    fn sum(&mut self) -> Result<Expr, FormulaError> {
        let mut lhs = self.product()?;
        loop {
            let op = match self.peek() {
                Some('+') => Op::Add,
                Some('-') => Op::Sub,
                _ => return Ok(lhs),
            };
            self.at += 1;
            let rhs = self.product()?;
            lhs = Expr::Bin(op, Box::new(lhs), Box::new(rhs));
        }
    }

    fn product(&mut self) -> Result<Expr, FormulaError> {
        let mut lhs = self.unary()?;
        loop {
            let op = match self.peek() {
                Some('*') => Op::Mul,
                Some('/') => Op::Div,
                _ => return Ok(lhs),
            };
            self.at += 1;
            let rhs = self.unary()?;
            lhs = Expr::Bin(op, Box::new(lhs), Box::new(rhs));
        }
    }

    fn unary(&mut self) -> Result<Expr, FormulaError> {
        match self.peek() {
            Some('-') => {
                self.at += 1;
                Ok(Expr::Neg(Box::new(self.unary()?)))
            }
            Some('+') => {
                self.at += 1;
                self.unary()
            }
            _ => self.atom(),
        }
    }

    fn atom(&mut self) -> Result<Expr, FormulaError> {
        match self.peek() {
            Some('(') => {
                self.at += 1;
                let inner = self.sum()?;
                match self.peek() {
                    Some(')') => {
                        self.at += 1;
                        Ok(inner)
                    }
                    Some(c) => Err(self.unexpected(c)),
                    None => Err(self.end()),
                }
            }
            Some(c) if c.is_ascii_digit() || c == '.' => self.number(),
            Some(c) if starts_name(c) => self.name(),
            Some(c) => Err(self.unexpected(c)),
            None => Err(self.end()),
        }
    }

    fn number(&mut self) -> Result<Expr, FormulaError> {
        let rest = &self.source[self.at..];
        let bytes = rest.as_bytes();
        let mut end = digits(bytes, 0);
        if bytes.get(end) == Some(&b'.') {
            end = digits(bytes, end + 1);
        }
        if matches!(bytes.get(end), Some(b'e' | b'E')) {
            let mut exponent = end + 1;
            if matches!(bytes.get(exponent), Some(b'+' | b'-')) {
                exponent += 1;
            }
            let after = digits(bytes, exponent);
            if after > exponent {
                end = after;
            }
        }
        let text = &rest[..end];
        let value = text.parse::<f64>().map_err(|_| FormulaError::Syntax {
            at: self.at,
            found: format!("`{text}`"),
        })?;
        self.at += end;
        Ok(Expr::Num(value))
    }

    fn name(&mut self) -> Result<Expr, FormulaError> {
        let rest = &self.source[self.at..];
        let end = rest
            .char_indices()
            .skip(1)
            .find(|&(_, c)| !continues_name(c))
            .map_or(rest.len(), |(i, _)| i);
        let name = &rest[..end];
        self.at += end;
        if self.peek() == Some('(') {
            return Err(FormulaError::Unsupported(format!(
                "the function call `{name}(`"
            )));
        }
        Ok(Expr::Name(name.to_owned()))
    }

    fn unexpected(&self, c: char) -> FormulaError {
        let what = match c {
            '^' => "the power `^`",
            '?' | ':' => "a conditional",
            '<' | '>' | '=' | '!' | '&' | '|' => "a comparison",
            ',' | ';' => "an argument list",
            _ => {
                return FormulaError::Syntax {
                    at: self.at,
                    found: format!("`{c}`"),
                };
            }
        };
        FormulaError::Unsupported(what.to_owned())
    }

    fn end(&self) -> FormulaError {
        FormulaError::Syntax {
            at: self.at,
            found: "the end of the formula".to_owned(),
        }
    }
}

fn digits(bytes: &[u8], from: usize) -> usize {
    let mut at = from;
    while bytes.get(at).is_some_and(u8::is_ascii_digit) {
        at += 1;
    }
    at
}

// `#` opens a pattern variable and `@` a custom measurement; the rest are the
// identifiers a Seamly name is made of, accents included.
fn starts_name(c: char) -> bool {
    c == '#' || c == '@' || c == '_' || c.is_alphabetic()
}

fn continues_name(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}
