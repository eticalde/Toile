use std::collections::BTreeMap;

use thiserror::Error;

use super::ast::{Cmp, Expr, Func, Op};

/// Where a formula's names get their values, in centimetres.
pub trait Lookup {
    /// The value bound to `name`, if the environment binds it.
    fn value(&self, name: &str) -> Option<f64>;
}

impl Lookup for BTreeMap<String, f64> {
    fn value(&self, name: &str) -> Option<f64> {
        self.get(name).copied()
    }
}

/// What can go wrong while giving a formula a value.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EvalError {
    /// A name the environment does not bind. Never a zero, never a stale value.
    #[error("unknown name `{0}`")]
    UnknownName(String),
    /// A division whose divisor is zero.
    #[error("division by zero")]
    DivideByZero,
    /// An exponent that is not a whole number.
    #[error("an exponent must be a whole number")]
    FractionalPower,
    /// A value that is not a finite number.
    #[error("the result is not a finite number")]
    NotFinite,
    /// Pattern variables that depend on each other.
    #[error("these variables depend on each other: {0}")]
    Cycle(String),
}

/// The value of `expr` in `env`.
///
/// Every step is taken in the order the tree holds it: no reassociation, no
/// algebraic shortcut. The result reaches the solver as a rest length, so a
/// rearrangement that is equal in algebra is a different drape in IEEE 754.
pub fn eval(expr: &Expr, env: &dyn Lookup) -> Result<f64, EvalError> {
    let value = match expr {
        Expr::Num(value) => *value,
        Expr::Name(name) => env
            .value(name)
            .ok_or_else(|| EvalError::UnknownName(name.clone()))?,
        Expr::Neg(inner) => -eval(inner, env)?,
        Expr::Bin(op, lhs, rhs) => binary(*op, eval(lhs, env)?, eval(rhs, env)?)?,
        Expr::Call(func, args) => call(*func, args, env)?,
        Expr::Cond {
            lhs,
            cmp,
            rhs,
            yes,
            no,
        } => {
            let taken = if holds(*cmp, eval(lhs, env)?, eval(rhs, env)?) {
                yes
            } else {
                no
            };
            eval(taken, env)?
        }
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err(EvalError::NotFinite)
    }
}

fn binary(op: Op, lhs: f64, rhs: f64) -> Result<f64, EvalError> {
    Ok(match op {
        Op::Add => lhs + rhs,
        Op::Sub => lhs - rhs,
        Op::Mul => lhs * rhs,
        Op::Div => {
            if rhs == 0.0 {
                return Err(EvalError::DivideByZero);
            }
            lhs / rhs
        }
        Op::Pow => {
            if rhs.fract() != 0.0 {
                return Err(EvalError::FractionalPower);
            }
            // An exponent past i32 saturates on the cast and overflows to a
            // non-finite power, which the caller reports.
            lhs.powi(rhs as i32)
        }
    })
}

fn call(func: Func, args: &[Expr], env: &dyn Lookup) -> Result<f64, EvalError> {
    let first = eval(nth(args, 0), env)?;
    Ok(match func {
        Func::Abs => first.abs(),
        Func::Sqrt => first.sqrt(),
        Func::Min => first.min(eval(nth(args, 1), env)?),
        Func::Max => first.max(eval(nth(args, 1), env)?),
    })
}

fn nth(args: &[Expr], index: usize) -> &Expr {
    args.get(index)
        .expect("the parser checks every call against Func::arity")
}

#[allow(
    clippy::float_cmp,
    reason = "the language's `==` is the user's, and it compares the two values as they are"
)]
fn holds(cmp: Cmp, lhs: f64, rhs: f64) -> bool {
    match cmp {
        Cmp::Lt => lhs < rhs,
        Cmp::Le => lhs <= rhs,
        Cmp::Gt => lhs > rhs,
        Cmp::Ge => lhs >= rhs,
        Cmp::Eq => lhs == rhs,
        Cmp::Ne => lhs != rhs,
    }
}

#[cfg(test)]
mod tests {
    use super::super::parse::parse;
    use super::*;

    fn env(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
        pairs
            .iter()
            .map(|&(name, value)| (name.to_owned(), value))
            .collect()
    }

    fn value(src: &str, pairs: &[(&str, f64)]) -> Result<f64, EvalError> {
        eval(&parse(src).expect("the source parses"), &env(pairs))
    }

    #[test]
    fn an_unknown_name_is_an_error_not_zero() {
        assert_eq!(
            value("cintura / 4", &[]),
            Err(EvalError::UnknownName("cintura".to_owned()))
        );
    }

    #[test]
    fn division_by_zero_is_an_error() {
        assert_eq!(value("1 / 0", &[]), Err(EvalError::DivideByZero));
        assert_eq!(
            value("1 / holgura", &[("holgura", -0.0)]),
            Err(EvalError::DivideByZero)
        );
    }

    #[test]
    fn a_fractional_power_is_rejected() {
        assert_eq!(value("2 ^ 0.5", &[]), Err(EvalError::FractionalPower));
        assert_eq!(value("2 ^ (1 / 2)", &[]), Err(EvalError::FractionalPower));
    }

    #[test]
    fn a_whole_power_is_taken_by_repeated_multiplication() {
        assert_eq!(value("1.1 ^ 3", &[]), Ok(1.1_f64.powi(3)));
        assert_eq!(value("2 ^ -2", &[]), Ok(0.25));
        assert_eq!(value("2 ^ 0", &[]), Ok(1.0));
    }

    #[test]
    fn a_result_that_is_not_finite_is_an_error() {
        assert_eq!(value("sqrt(-1)", &[]), Err(EvalError::NotFinite));
        assert_eq!(value("10 ^ 400", &[]), Err(EvalError::NotFinite));
    }

    #[test]
    fn a_ternary_selects_without_evaluating_both() {
        assert_eq!(value("(1 < 2 ? 3 : 1 / 0)", &[]), Ok(3.0));
        assert_eq!(value("(1 > 2 ? 1 / 0 : 3)", &[]), Ok(3.0));
        assert_eq!(value("(1 != 1 ? 5 : 6)", &[]), Ok(6.0));
    }

    #[test]
    fn addition_runs_left_to_right_as_written() {
        let big = 1.0e16;
        let left = value("a + b + c", &[("a", big), ("b", 1.0), ("c", 1.0)]);
        assert_eq!(left, Ok((big + 1.0) + 1.0));
        let right = value("a + (b + c)", &[("a", big), ("b", 1.0), ("c", 1.0)]);
        assert_eq!(right, Ok(big + (1.0 + 1.0)));
        assert_ne!(left, right);
    }

    #[test]
    fn the_functions_are_the_four_the_language_has() {
        assert_eq!(value("min(3, 4)", &[]), Ok(3.0));
        assert_eq!(value("max(3, 4)", &[]), Ok(4.0));
        assert_eq!(value("abs(0 - 3)", &[]), Ok(3.0));
        assert_eq!(value("sqrt(9)", &[]), Ok(3.0));
    }

    #[test]
    fn a_side_seam_coordinate_resolves_against_a_mannequin() {
        let pairs = &[("cadera", 98.0), ("holgura_cadera", 1.0)];
        assert_eq!(value("cadera / 4 + holgura_cadera", pairs), Ok(25.5));
    }
}
