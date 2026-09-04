use std::collections::BTreeSet;

/// A parsed formula: numbers, names, and the operations between them.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A decimal literal, in centimetres.
    Num(f64),
    /// A measurement or a pattern variable, by name.
    Name(String),
    /// A negated expression.
    Neg(Box<Expr>),
    /// Two operands and the operator between them.
    Bin(Op, Box<Expr>, Box<Expr>),
    /// A call to one of the language's four functions.
    Call(Func, Vec<Expr>),
    /// A comparison that chooses between two expressions.
    Cond {
        /// Left side of the comparison.
        lhs: Box<Expr>,
        /// The comparison itself.
        cmp: Cmp,
        /// Right side of the comparison.
        rhs: Box<Expr>,
        /// The value taken when the comparison holds.
        yes: Box<Expr>,
        /// The value taken when it does not.
        no: Box<Expr>,
    },
}

impl Expr {
    /// Adds every name this expression reads to `out`.
    pub fn collect_names<'a>(&'a self, out: &mut BTreeSet<&'a str>) {
        match self {
            Expr::Num(_) => {}
            Expr::Name(name) => {
                out.insert(name);
            }
            Expr::Neg(inner) => inner.collect_names(out),
            Expr::Bin(_, lhs, rhs) => {
                lhs.collect_names(out);
                rhs.collect_names(out);
            }
            Expr::Call(_, args) => {
                for arg in args {
                    arg.collect_names(out);
                }
            }
            Expr::Cond {
                lhs, rhs, yes, no, ..
            } => {
                for part in [lhs, rhs, yes, no] {
                    part.collect_names(out);
                }
            }
        }
    }
}

/// A binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Addition.
    Add,
    /// Subtraction.
    Sub,
    /// Multiplication.
    Mul,
    /// Division.
    Div,
    /// Raising to a whole exponent.
    Pow,
}

/// The functions the language has, and it has no others.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    /// The smaller of two values.
    Min,
    /// The larger of two values.
    Max,
    /// Distance from zero.
    Abs,
    /// Square root, exact under IEEE 754.
    Sqrt,
}

impl Func {
    /// The function this name spells, if it spells one.
    pub fn from_name(name: &str) -> Option<Func> {
        match name {
            "min" => Some(Func::Min),
            "max" => Some(Func::Max),
            "abs" => Some(Func::Abs),
            "sqrt" => Some(Func::Sqrt),
            _ => None,
        }
    }

    /// How many arguments the function takes.
    pub fn arity(self) -> usize {
        match self {
            Func::Min | Func::Max => 2,
            Func::Abs | Func::Sqrt => 1,
        }
    }
}

/// A comparison, which exists only as the condition of a conditional.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmp {
    /// Less than.
    Lt,
    /// Less than or equal.
    Le,
    /// Greater than.
    Gt,
    /// Greater than or equal.
    Ge,
    /// Equal.
    Eq,
    /// Not equal.
    Ne,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(expr: &Expr) -> Vec<String> {
        let mut out = BTreeSet::new();
        expr.collect_names(&mut out);
        out.into_iter().map(str::to_owned).collect()
    }

    #[test]
    fn a_literal_reads_no_names() {
        assert!(names(&Expr::Num(1.0)).is_empty());
    }

    #[test]
    fn every_function_name_maps_back_to_itself() {
        for (name, func) in [
            ("min", Func::Min),
            ("max", Func::Max),
            ("abs", Func::Abs),
            ("sqrt", Func::Sqrt),
        ] {
            assert_eq!(Func::from_name(name), Some(func));
        }
        assert_eq!(Func::from_name("cintura"), None);
    }

    #[test]
    fn min_takes_two_arguments_and_abs_takes_one() {
        assert_eq!(Func::Min.arity(), 2);
        assert_eq!(Func::Max.arity(), 2);
        assert_eq!(Func::Abs.arity(), 1);
        assert_eq!(Func::Sqrt.arity(), 1);
    }

    #[test]
    fn a_name_repeated_appears_once() {
        let expr = Expr::Bin(
            Op::Add,
            Box::new(Expr::Name("cadera".to_owned())),
            Box::new(Expr::Name("cadera".to_owned())),
        );
        assert_eq!(names(&expr), vec!["cadera"]);
    }

    #[test]
    fn a_conditional_reads_all_four_of_its_parts() {
        let expr = Expr::Cond {
            lhs: Box::new(Expr::Name("a".to_owned())),
            cmp: Cmp::Lt,
            rhs: Box::new(Expr::Name("b".to_owned())),
            yes: Box::new(Expr::Name("c".to_owned())),
            no: Box::new(Expr::Call(Func::Abs, vec![Expr::Name("d".to_owned())])),
        };
        assert_eq!(names(&expr), vec!["a", "b", "c", "d"]);
    }
}
