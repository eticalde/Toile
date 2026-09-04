use super::ast::{Expr, Func, Op};
use super::cursor::Cursor;
use super::lex::{self, TokenKind};
use super::syntax::{SyntaxError, SyntaxKind};

/// Most tokens a formula may be written with.
///
/// A sum is parsed by a loop, so nesting alone does not bound the tree it
/// builds: `1+1+1...` stays one level deep and grows without limit, and every
/// walk of the tree afterwards — evaluating it, cloning it into the undo
/// stack, dropping it — is recursive.
const MAX_TOKENS: usize = 1024;

/// Reads a formula's source text into an expression.
pub fn parse(src: &str) -> Result<Expr, SyntaxError> {
    let tokens = lex::tokenize(src)?;
    if tokens.is_empty() {
        return Err(SyntaxError::new(0, SyntaxKind::Empty));
    }
    if tokens.len() > MAX_TOKENS {
        return Err(SyntaxError::new(tokens[MAX_TOKENS].at, SyntaxKind::TooLong));
    }
    let mut cur = Cursor::new(&tokens, src.len());
    let expr = expression(&mut cur)?;
    if !cur.done() {
        return Err(cur.error(SyntaxKind::TrailingInput));
    }
    Ok(expr)
}

/// A sum, or a comparison choosing between two expressions.
fn expression(cur: &mut Cursor<'_>) -> Result<Expr, SyntaxError> {
    cur.deepen()?;
    let lhs = sum(cur)?;
    let expr = match cur.peek() {
        Some(&TokenKind::Cmp(cmp)) => {
            cur.bump();
            let rhs = sum(cur)?;
            if !cur.eat(&TokenKind::Question) {
                return Err(cur.error(SyntaxKind::ExpectedQuestionMark));
            }
            let yes = expression(cur)?;
            if !cur.eat(&TokenKind::Colon) {
                return Err(cur.error(SyntaxKind::ExpectedColon));
            }
            Expr::Cond {
                lhs: Box::new(lhs),
                cmp,
                rhs: Box::new(rhs),
                yes: Box::new(yes),
                no: Box::new(expression(cur)?),
            }
        }
        Some(TokenKind::Question) => return Err(cur.error(SyntaxKind::ExpectedComparison)),
        _ => lhs,
    };
    cur.shallow();
    Ok(expr)
}

fn sum(cur: &mut Cursor<'_>) -> Result<Expr, SyntaxError> {
    let mut lhs = product(cur)?;
    while let Some(&TokenKind::Op(op @ (Op::Add | Op::Sub))) = cur.peek() {
        cur.bump();
        lhs = Expr::Bin(op, Box::new(lhs), Box::new(product(cur)?));
    }
    Ok(lhs)
}

fn product(cur: &mut Cursor<'_>) -> Result<Expr, SyntaxError> {
    let mut lhs = unary(cur)?;
    while let Some(&TokenKind::Op(op @ (Op::Mul | Op::Div))) = cur.peek() {
        cur.bump();
        lhs = Expr::Bin(op, Box::new(lhs), Box::new(unary(cur)?));
    }
    Ok(lhs)
}

fn unary(cur: &mut Cursor<'_>) -> Result<Expr, SyntaxError> {
    cur.deepen()?;
    let expr = if cur.eat(&TokenKind::Op(Op::Sub)) {
        Expr::Neg(Box::new(unary(cur)?))
    } else {
        power(cur)?
    };
    cur.shallow();
    Ok(expr)
}

/// Right associative, and its exponent may be negated.
fn power(cur: &mut Cursor<'_>) -> Result<Expr, SyntaxError> {
    let base = atom(cur)?;
    if !cur.eat(&TokenKind::Op(Op::Pow)) {
        return Ok(base);
    }
    Ok(Expr::Bin(Op::Pow, Box::new(base), Box::new(unary(cur)?)))
}

fn atom(cur: &mut Cursor<'_>) -> Result<Expr, SyntaxError> {
    let at = cur.at();
    let Some(kind) = cur.peek() else {
        return Err(cur.error(SyntaxKind::ExpectedExpression));
    };
    match kind {
        &TokenKind::Number(value) => {
            cur.bump();
            Ok(Expr::Num(value))
        }
        TokenKind::Name(name) => {
            let name = name.clone();
            cur.bump();
            if cur.peek() == Some(&TokenKind::LParen) {
                return Err(SyntaxError::new(at, SyntaxKind::UnknownFunction(name)));
            }
            Ok(Expr::Name(name))
        }
        &TokenKind::Func(func) => {
            cur.bump();
            call(cur, func, at)
        }
        TokenKind::LParen => {
            cur.bump();
            let inner = expression(cur)?;
            if !cur.eat(&TokenKind::RParen) {
                return Err(cur.error(SyntaxKind::ExpectedClosingParen));
            }
            Ok(inner)
        }
        _ => Err(cur.error(SyntaxKind::ExpectedExpression)),
    }
}

fn call(cur: &mut Cursor<'_>, func: Func, at: usize) -> Result<Expr, SyntaxError> {
    if !cur.eat(&TokenKind::LParen) {
        return Err(cur.error(SyntaxKind::ExpectedOpeningParen));
    }
    let mut args = vec![expression(cur)?];
    while cur.eat(&TokenKind::Comma) {
        args.push(expression(cur)?);
    }
    if !cur.eat(&TokenKind::RParen) {
        return Err(cur.error(SyntaxKind::ExpectedClosingParen));
    }
    if args.len() == func.arity() {
        return Ok(Expr::Call(func, args));
    }
    let kind = SyntaxKind::WrongArgumentCount {
        expected: func.arity(),
        found: args.len(),
    };
    Err(SyntaxError::new(at, kind))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tree in prefix form, where nesting is precedence made visible.
    fn show(src: &str) -> String {
        render(&parse(src).expect("the source parses"))
    }

    fn fault(src: &str) -> SyntaxKind {
        parse(src).unwrap_err().kind
    }

    fn render(expr: &Expr) -> String {
        let list = |head: &str, parts: &[&Expr]| {
            let rendered: Vec<String> = parts.iter().copied().map(render).collect();
            format!("({head} {})", rendered.join(" "))
        };
        match expr {
            Expr::Num(value) => format!("{value}"),
            Expr::Name(name) => name.clone(),
            Expr::Neg(inner) => list("-", &[&**inner]),
            Expr::Bin(op, lhs, rhs) => list(&format!("{op:?}"), &[&**lhs, &**rhs]),
            Expr::Call(func, args) => {
                let refs: Vec<&Expr> = args.iter().collect();
                list(&format!("{func:?}"), &refs)
            }
            Expr::Cond {
                lhs,
                cmp,
                rhs,
                yes,
                no,
            } => list(&format!("{cmp:?}"), &[&**lhs, &**rhs, &**yes, &**no]),
        }
    }

    #[test]
    fn an_empty_formula_is_an_error() {
        assert_eq!(fault(""), SyntaxKind::Empty);
        assert_eq!(fault("  "), SyntaxKind::Empty);
    }

    #[test]
    fn multiplication_binds_tighter_than_addition() {
        assert_eq!(show("1 + 2 * 3"), "(Add 1 (Mul 2 3))");
        assert_eq!(show("1 * 2 + 3"), "(Add (Mul 1 2) 3)");
    }

    #[test]
    fn addition_associates_to_the_left() {
        assert_eq!(show("1 + 2 + 3"), "(Add (Add 1 2) 3)");
        assert_eq!(show("8 / 4 / 2"), "(Div (Div 8 4) 2)");
    }

    #[test]
    fn parentheses_override_precedence() {
        assert_eq!(show("(1 + 2) * 3"), "(Mul (Add 1 2) 3)");
    }

    #[test]
    fn a_power_binds_tighter_than_the_unary_minus_before_it() {
        assert_eq!(show("-2 ^ 2"), "(- (Pow 2 2))");
        assert_eq!(show("-cintura / 4"), "(Div (- cintura) 4)");
    }

    #[test]
    fn power_associates_to_the_right() {
        assert_eq!(show("2 ^ 3 ^ 2"), "(Pow 2 (Pow 3 2))");
        assert_eq!(show("2 ^ -1"), "(Pow 2 (- 1))");
    }

    #[test]
    fn a_conditional_takes_a_comparison_and_two_arms() {
        assert_eq!(show("(cadera < 90 ? 1 : 2)"), "(Lt cadera 90 1 2)");
        assert_eq!(fault("(cadera ? 1 : 2)"), SyntaxKind::ExpectedComparison);
        assert_eq!(fault("cadera < 90"), SyntaxKind::ExpectedQuestionMark);
        assert_eq!(fault("cadera < 90 ? 1"), SyntaxKind::ExpectedColon);
    }

    #[test]
    fn an_unclosed_paren_reports_its_offset() {
        let err = parse("(1 + 2").unwrap_err();
        assert_eq!(err.at, 6);
        assert_eq!(err.kind, SyntaxKind::ExpectedClosingParen);
    }

    #[test]
    fn the_argument_separator_is_a_comma() {
        assert_eq!(show("min(1, 2)"), "(Min 1 2)");
        assert_eq!(fault("min(1; 2)"), SyntaxKind::UnexpectedChar(';'));
    }

    #[test]
    fn a_call_with_the_wrong_argument_count_is_rejected() {
        let expected = SyntaxKind::WrongArgumentCount {
            expected: 1,
            found: 2,
        };
        assert_eq!(fault("abs(1, 2)"), expected);
    }

    #[test]
    fn an_unknown_function_is_rejected_at_its_name() {
        let err = parse("1 + pow(2, 3)").unwrap_err();
        assert_eq!(err.at, 4);
        assert_eq!(err.kind, SyntaxKind::UnknownFunction("pow".to_owned()));
    }

    #[test]
    fn a_function_name_without_a_call_is_rejected() {
        assert_eq!(fault("min + 1"), SyntaxKind::ExpectedOpeningParen);
    }

    #[test]
    fn input_after_the_expression_is_rejected() {
        assert_eq!(fault("1 2"), SyntaxKind::TrailingInput);
        assert_eq!(fault("(1) )"), SyntaxKind::TrailingInput);
    }

    #[test]
    fn a_missing_operand_is_rejected() {
        assert_eq!(fault("1 +"), SyntaxKind::ExpectedExpression);
    }

    #[test]
    fn a_long_flat_sum_is_rejected_instead_of_overflowing_the_stack() {
        // `1+1+1...` stays one level deep however long it grows, so only the
        // token count catches it. One token per byte here, so `at` is a count.
        let flat = |terms: usize| "1".to_owned() + &"+1".repeat(terms);
        assert!(parse(&flat((MAX_TOKENS - 1) / 2)).is_ok());
        let err = parse(&flat(MAX_TOKENS)).unwrap_err();
        assert_eq!(err.kind, SyntaxKind::TooLong);
        assert_eq!(err.at, MAX_TOKENS);
    }

    #[test]
    fn deep_nesting_is_rejected_instead_of_overflowing_the_stack() {
        let nested = format!("{}1{}", "(".repeat(500), ")".repeat(500));
        assert_eq!(fault(&nested), SyntaxKind::TooDeep);
        assert_eq!(fault(&format!("{}1", "-".repeat(500))), SyntaxKind::TooDeep);
    }
}
