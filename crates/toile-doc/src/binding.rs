use std::borrow::Cow;
use std::collections::BTreeSet;

use crate::formula::{EvalError, Expr, Formula, Lookup, SyntaxError};

/// What a coordinate is bound to: a number, or an expression over the
/// measurements and the pattern variables.
///
/// The literal is the degenerate formula, not a mode of its own: that is what
/// keeps a pattern parametric after a drag instead of before it. It stays a
/// variant so that dragging a plain number does not allocate a string.
///
/// The resolved value is never written back into a binding. Only a gesture of
/// the user writes one, and it writes what the user meant, never what the
/// measurements made of it.
#[derive(Debug, Clone, PartialEq)]
pub enum Binding {
    /// A number, in centimetres.
    Literal(f64),
    /// An expression, in centimetres.
    Formula(Formula),
}

impl Binding {
    /// The binding a plain number makes.
    pub fn literal(value: f64) -> Binding {
        Binding::Literal(value)
    }

    /// The binding some source text makes.
    ///
    /// Source that spells nothing but a number becomes a literal, so that the
    /// simple case stays simple wherever it was typed.
    ///
    /// # Errors
    /// `SyntaxError`, carrying the byte offset where the source stops parsing.
    pub fn parse(src: &str) -> Result<Binding, SyntaxError> {
        let formula = Formula::parse(src)?;
        match formula.expr() {
            Expr::Num(value) => Ok(Binding::Literal(*value)),
            _ => Ok(Binding::Formula(formula)),
        }
    }

    /// The text the mono field shows, exactly as it was written.
    pub fn source(&self) -> Cow<'_, str> {
        match self {
            Binding::Literal(value) => Cow::Owned(format!("{value}")),
            Binding::Formula(formula) => Cow::Borrowed(formula.source()),
        }
    }

    /// The value of the binding in `env`, in centimetres.
    ///
    /// # Errors
    /// `EvalError` for an unknown name, a zero divisor, a fractional exponent
    /// or a result that is not finite.
    pub fn eval(&self, env: &dyn Lookup) -> Result<f64, EvalError> {
        match self {
            Binding::Literal(value) => Ok(*value),
            Binding::Formula(formula) => formula.eval(env),
        }
    }

    /// Every name the binding reads, measurements and variables alike.
    pub fn names(&self) -> BTreeSet<&str> {
        match self {
            Binding::Literal(_) => BTreeSet::new(),
            Binding::Formula(formula) => formula.names(),
        }
    }

    /// Whether the binding is a plain number.
    pub fn is_literal(&self) -> bool {
        matches!(self, Binding::Literal(_))
    }
}

impl From<f64> for Binding {
    fn from(value: f64) -> Binding {
        Binding::Literal(value)
    }
}

impl From<Formula> for Binding {
    fn from(formula: Formula) -> Binding {
        Binding::Formula(formula)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, reason = "a literal stores the number it was given")]

    use std::collections::BTreeMap;

    use super::*;

    fn env(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
        pairs
            .iter()
            .map(|&(name, value)| (name.to_owned(), value))
            .collect()
    }

    #[test]
    fn a_number_parses_to_a_literal() {
        assert_eq!(Binding::parse("22"), Ok(Binding::Literal(22.0)));
        assert_eq!(Binding::parse(" 22.5 "), Ok(Binding::Literal(22.5)));
    }

    #[test]
    fn an_expression_parses_to_a_formula() {
        let binding = Binding::parse("cintura / 4 + 1").expect("the source parses");
        assert!(!binding.is_literal());
        assert_eq!(binding.source(), "cintura / 4 + 1");
        assert_eq!(binding.eval(&env(&[("cintura", 84.0)])), Ok(22.0));
    }

    #[test]
    fn a_literal_reads_back_as_the_number_it_holds() {
        assert_eq!(Binding::literal(20.875).source(), "20.875");
        assert_eq!(Binding::literal(0.0).source(), "0");
    }

    #[test]
    fn a_literal_reads_no_name() {
        assert!(Binding::literal(1.0).names().is_empty());
        assert_eq!(Binding::literal(1.0).eval(&env(&[])), Ok(1.0));
    }

    #[test]
    fn a_formula_reads_the_names_it_mentions() {
        let binding = Binding::parse("raya + ancho_bajo / 2").expect("the source parses");
        assert_eq!(
            binding.names().into_iter().collect::<Vec<_>>(),
            ["ancho_bajo", "raya"]
        );
    }

    #[test]
    fn an_unknown_name_stops_the_binding() {
        let binding = Binding::parse("cintura + 1").expect("the source parses");
        assert_eq!(
            binding.eval(&env(&[])),
            Err(EvalError::UnknownName("cintura".to_owned()))
        );
    }
}
