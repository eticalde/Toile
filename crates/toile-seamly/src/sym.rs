use std::collections::BTreeSet;

use crate::Id;

/// Products, quotients and roots of linear forms.
mod nonlinear;
/// Coefficients that stay exact while they can.
mod num;
/// Exact rationals.
mod rat;
/// A linear form as the source text of a Toile formula.
mod render;

pub(crate) use num::Num;
pub(crate) use rat::Rat;

/// A quantity the translation could not keep parametric and wrote as the
/// number it had for the imported body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Frozen {
    /// How much longer than its chord the curved spline with this id is. The
    /// chord follows the body; the excess is the imported body's.
    SplineExcess(Id),
    /// Where on its spline the cut point with this id falls, as the curve's
    /// parameter. The curve follows the body; the parameter does not, so the
    /// cut no longer lands at the arc length its formula asks for.
    CutParameter(Id),
}

/// A term of a linear form that is not a plain number: a measurement or
/// variable, or an expression the form cannot open up.
///
/// Two atoms are the same atom when they print the same, which is what lets
/// a difference of two coordinates built on the same point cancel exactly.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Atom {
    shape: Shape,
    source: String,
}

/// What an atom is, which decides how it is parenthesised and simplified.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Shape {
    /// A measurement or a variable.
    Name,
    /// `sqrt(…)` or `abs(…)`.
    Call,
    /// A square, with the source of what is squared, so that the root of a
    /// square becomes an absolute value.
    Square(String),
    /// Factors joined by `*` and `/`.
    Product,
}

/// A number plus a weighted sum of atoms: the form every coordinate of a
/// drafting construction takes, as long as it squares off.
///
/// Sums of offsets simplify exactly — `(36 + a) - 36` is `a`, and the height
/// difference of two points on one horizontal line is zero, not a number near
/// it. Anything the form cannot open up becomes an atom, kept whole.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Lin {
    exact: Rat,
    /// The part of the constant that could not be held exactly.
    float: f64,
    /// Sorted by atom, no coefficient zero.
    terms: Vec<(Atom, Num)>,
    /// Every frozen quantity the value depends on.
    frozen: BTreeSet<Frozen>,
}

impl Lin {
    pub(crate) fn zero() -> Lin {
        Lin::num(Num::Exact(Rat::ZERO))
    }

    pub(crate) fn num(value: Num) -> Lin {
        let (exact, float) = match value {
            Num::Exact(rat) => (rat, 0.0),
            Num::Float(value) => (Rat::ZERO, value),
        };
        Lin {
            exact,
            float,
            terms: Vec::new(),
            frozen: BTreeSet::new(),
        }
    }

    /// A measurement or a variable, by the name a Toile formula reads it by.
    pub(crate) fn name(name: &str) -> Lin {
        Lin::atom(Shape::Name, name.to_owned())
    }

    fn atom(shape: Shape, source: String) -> Lin {
        let mut lin = Lin::zero();
        lin.terms.push((Atom { shape, source }, Num::ONE));
        lin
    }

    /// The same value, marked as depending on `frozen`.
    pub(crate) fn frozen_by(mut self, frozen: Frozen) -> Lin {
        self.frozen.insert(frozen);
        self
    }

    pub(crate) fn frozen(&self) -> &BTreeSet<Frozen> {
        &self.frozen
    }

    /// The value, when it is a plain number.
    pub(crate) fn constant(&self) -> Option<Num> {
        if !self.terms.is_empty() {
            return None;
        }
        if self.has_float() {
            Some(Num::Float(self.exact.to_f64() + self.float))
        } else {
            Some(Num::Exact(self.exact))
        }
    }

    /// Whether some of the constant could not be held exactly.
    #[allow(
        clippy::float_cmp,
        reason = "the inexact part is absent exactly when it is zero"
    )]
    fn has_float(&self) -> bool {
        self.float != 0.0
    }

    pub(crate) fn is_zero(&self) -> bool {
        self.constant().is_some_and(Num::is_zero)
    }

    /// Whether every atom is a plain name: a coordinate that squares off.
    pub(crate) fn is_linear(&self) -> bool {
        self.terms.iter().all(|(atom, _)| atom.shape == Shape::Name)
    }

    /// The one atom and its weight, when the form is nothing else.
    fn single(&self) -> Option<(&Atom, Num)> {
        match self.terms.as_slice() {
            [(atom, weight)] if self.exact.is_zero() && !self.has_float() => Some((atom, *weight)),
            _ => None,
        }
    }

    fn with_frozen(mut self, a: &Lin, b: &Lin) -> Lin {
        self.frozen.extend(a.frozen.iter().copied());
        self.frozen.extend(b.frozen.iter().copied());
        self
    }

    pub(crate) fn add(&self, other: &Lin) -> Lin {
        let mut sum = self.clone();
        if let Some(exact) = self.exact.add(other.exact) {
            sum.exact = exact;
        } else {
            sum.exact = Rat::ZERO;
            sum.float += self.exact.to_f64() + other.exact.to_f64();
        }
        sum.float += other.float;
        for (atom, weight) in &other.terms {
            match sum.terms.binary_search_by(|(held, _)| held.cmp(atom)) {
                Ok(at) => {
                    let total = sum.terms[at].1.add(*weight);
                    if total.is_zero() {
                        sum.terms.remove(at);
                    } else {
                        sum.terms[at].1 = total;
                    }
                }
                Err(at) => sum.terms.insert(at, (atom.clone(), *weight)),
            }
        }
        sum.frozen.extend(other.frozen.iter().copied());
        sum
    }

    pub(crate) fn sub(&self, other: &Lin) -> Lin {
        self.add(&other.neg())
    }

    pub(crate) fn neg(&self) -> Lin {
        self.scale(Num::ONE.neg())
    }

    /// The form times a number. Times an exact zero it is zero, and depends
    /// on nothing.
    pub(crate) fn scale(&self, by: Num) -> Lin {
        if by == Num::Exact(Rat::ZERO) {
            return Lin::zero();
        }
        let mut scaled = self.clone();
        let exact = match by {
            Num::Exact(rat) => self.exact.mul(rat),
            Num::Float(_) => None,
        };
        if let Some(exact) = exact {
            scaled.exact = exact;
            scaled.float = if self.has_float() {
                self.float * by.value()
            } else {
                0.0
            };
        } else {
            scaled.exact = Rat::ZERO;
            scaled.float = (self.exact.to_f64() + self.float) * by.value();
        }
        for (_, weight) in &mut scaled.terms {
            *weight = weight.mul(by);
        }
        scaled.terms.retain(|(_, weight)| !weight.is_zero());
        scaled
    }
}

#[cfg(test)]
mod tests;
