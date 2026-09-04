mod ast;
mod cursor;
mod eval;
mod lex;
mod order;
mod parse;
mod syntax;

use std::collections::BTreeSet;

pub use ast::{Cmp, Expr, Func, Op};
pub use eval::{EvalError, Lookup};
pub use order::{Dependency, evaluation_order};
pub use syntax::{SyntaxError, SyntaxKind};

/// A coordinate written as an expression over measurements and variables.
///
/// The source text is what the document stores and what a person, or a
/// language model, reads in the file: it survives save and load exactly as it
/// was typed, spacing included.
#[derive(Debug, Clone, PartialEq)]
pub struct Formula {
    src: String,
    expr: Expr,
}

impl Formula {
    /// Reads a formula from its source text, in centimetres.
    ///
    /// # Errors
    /// `SyntaxError`, carrying the byte offset where the source stops parsing.
    pub fn parse(src: &str) -> Result<Formula, SyntaxError> {
        Ok(Formula {
            src: src.to_owned(),
            expr: parse::parse(src)?,
        })
    }

    /// The source text, exactly as it was written.
    pub fn source(&self) -> &str {
        &self.src
    }

    /// The expression the source text spells.
    pub fn expr(&self) -> &Expr {
        &self.expr
    }

    /// The value of the formula in `env`, in centimetres.
    ///
    /// # Errors
    /// `EvalError` for an unknown name, a zero divisor, a fractional exponent
    /// or a result that is not finite.
    pub fn eval(&self, env: &dyn Lookup) -> Result<f64, EvalError> {
        eval::eval(&self.expr, env)
    }

    /// Every name the formula reads, measurements and variables alike.
    pub fn names(&self) -> BTreeSet<&str> {
        let mut out = BTreeSet::new();
        self.expr.collect_names(&mut out);
        out
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn etienne() -> BTreeMap<String, f64> {
        [
            ("cintura", 84.0),
            ("cadera", 98.0),
            ("tiro", 27.0),
            ("altura_cadera", 20.0),
            ("largo_lateral", 104.0),
        ]
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value))
        .collect()
    }

    #[test]
    fn the_source_is_kept_exactly_as_written() {
        let formula = Formula::parse("cintura /4  + 1").expect("the source parses");
        assert_eq!(formula.source(), "cintura /4  + 1");
    }

    #[test]
    fn a_literal_is_a_formula_like_any_other() {
        let formula = Formula::parse("0").expect("the source parses");
        assert!(formula.names().is_empty());
        assert_eq!(formula.eval(&BTreeMap::new()), Ok(0.0));
    }

    #[test]
    fn names_lists_every_identifier_once_in_order() {
        let formula = Formula::parse("cadera / 4 + holgura - cadera / 8").expect("it parses");
        assert_eq!(
            formula.names().into_iter().collect::<Vec<_>>(),
            ["cadera", "holgura"]
        );
    }

    #[test]
    fn a_waist_coordinate_resolves_against_a_mannequin() {
        let formula = Formula::parse("cintura / 4 + 1").expect("the source parses");
        assert_eq!(formula.eval(&etienne()), Ok(22.0));
    }

    #[test]
    fn the_variables_of_a_block_resolve_in_dependency_order() {
        let sources = [
            ("raya", "(cadera / 4 + holgura_cadera - extension_tiro) / 2"),
            ("holgura_cadera", "1"),
            ("extension_tiro", "cadera / 16"),
        ];
        let parsed: Vec<(&str, Formula)> = sources
            .iter()
            .map(|&(name, src)| (name, Formula::parse(src).expect("the source parses")))
            .collect();
        let graph: Vec<Dependency<'_>> = parsed
            .iter()
            .map(|(name, formula)| Dependency {
                name,
                reads: formula.names(),
            })
            .collect();

        let mut env = etienne();
        for index in evaluation_order(&graph).expect("the block has no cycle") {
            let (name, formula) = &parsed[index];
            let value = formula.eval(&env).expect("every name is bound by now");
            env.insert((*name).to_owned(), value);
        }
        assert_eq!(env.value("extension_tiro"), Some(98.0 / 16.0));
        assert_eq!(
            env.value("raya"),
            Some((98.0 / 4.0 + 1.0 - 98.0 / 16.0) / 2.0)
        );
    }

    #[test]
    fn an_unknown_name_stops_the_resolution() {
        let formula = Formula::parse("cintura + holgura").expect("the source parses");
        assert_eq!(
            formula.eval(&etienne()),
            Err(EvalError::UnknownName("holgura".to_owned()))
        );
    }
}
