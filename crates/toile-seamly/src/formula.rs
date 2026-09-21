use std::collections::BTreeSet;

use thiserror::Error;

/// Turning formula text into its tree.
mod parse;

/// A Seamly formula: the text as the file writes it, and the tree it parses
/// to.
///
/// The text is kept because an importer translates formulas as well as
/// evaluating them.
#[derive(Debug, Clone, PartialEq)]
pub struct Formula {
    source: String,
    expr: Expr,
}

/// The tree of a formula. Only the arithmetic the supported files use:
/// anything else fails to parse, naming what it found.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A decimal literal.
    Num(f64),
    /// A measurement, a pattern variable (`#name`), or a value a drawing
    /// defines (`Line_a_b`, `AngleLine_a_b`, `Spl_a_b`, `CurrentLength`).
    Name(String),
    /// Unary minus.
    Neg(Box<Expr>),
    /// A binary operation, left operand first.
    Bin(Op, Box<Expr>, Box<Expr>),
}

/// A binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
}

/// Why a formula did not parse or did not evaluate.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FormulaError {
    /// A token where the grammar allows none.
    #[error("unexpected {found} at byte {at}")]
    Syntax {
        /// Byte offset into the formula.
        at: usize,
        /// What stood there.
        found: String,
    },
    /// Valid Seamly this crate does not evaluate: a function call, a power,
    /// a comparison.
    #[error("{0} is not supported")]
    Unsupported(String),
    /// A name with no value at this point of the file.
    #[error("nothing named `{0}` is defined above this point")]
    UnknownName(String),
    /// A name two different drawings define with different values.
    #[error("`{0}` names two different drawings")]
    Ambiguous(String),
    /// Division by zero, or an overflow.
    #[error("the result is not a finite number")]
    NotFinite,
}

impl Formula {
    /// Parses a formula as a Seamly file writes it.
    ///
    /// # Errors
    ///
    /// [`FormulaError::Syntax`] for text outside the grammar, and
    /// [`FormulaError::Unsupported`] for Seamly syntax this crate does not
    /// model.
    pub fn parse(source: &str) -> Result<Self, FormulaError> {
        let expr = parse::parse(source)?;
        Ok(Self {
            source: source.to_owned(),
            expr,
        })
    }

    /// The formula as the file writes it.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The parsed tree.
    pub fn expr(&self) -> &Expr {
        &self.expr
    }

    /// Every name the formula cites.
    pub fn names(&self) -> BTreeSet<&str> {
        let mut names = BTreeSet::new();
        self.expr.collect(&mut names);
        names
    }

    /// Evaluates the formula, asking `lookup` for each name it cites.
    ///
    /// # Errors
    ///
    /// Whatever `lookup` returns for a name, and
    /// [`FormulaError::NotFinite`] when the arithmetic leaves the finite
    /// numbers.
    pub fn eval(
        &self,
        lookup: &dyn Fn(&str) -> Result<f64, FormulaError>,
    ) -> Result<f64, FormulaError> {
        let value = self.expr.eval(lookup)?;
        if value.is_finite() {
            Ok(value)
        } else {
            Err(FormulaError::NotFinite)
        }
    }
}

impl Expr {
    fn eval(
        &self,
        lookup: &dyn Fn(&str) -> Result<f64, FormulaError>,
    ) -> Result<f64, FormulaError> {
        Ok(match self {
            Self::Num(value) => *value,
            Self::Name(name) => lookup(name)?,
            Self::Neg(inner) => -inner.eval(lookup)?,
            Self::Bin(op, lhs, rhs) => {
                let (lhs, rhs) = (lhs.eval(lookup)?, rhs.eval(lookup)?);
                match op {
                    Op::Add => lhs + rhs,
                    Op::Sub => lhs - rhs,
                    Op::Mul => lhs * rhs,
                    Op::Div => lhs / rhs,
                }
            }
        })
    }

    fn collect<'a>(&'a self, names: &mut BTreeSet<&'a str>) {
        match self {
            Self::Num(_) => {}
            Self::Name(name) => {
                names.insert(name);
            }
            Self::Neg(inner) => inner.collect(names),
            Self::Bin(_, lhs, rhs) => {
                lhs.collect(names);
                rhs.collect(names);
            }
        }
    }
}
