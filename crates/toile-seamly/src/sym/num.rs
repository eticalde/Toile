use super::rat::Rat;

/// A coefficient: exact while the arithmetic allows, a float once a
/// trigonometric constant, a frozen length or an overflow has entered it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Num {
    /// A decimal of the file, or a ratio of them.
    Exact(Rat),
    /// A value computed once at import.
    Float(f64),
}

impl Num {
    pub(crate) const ONE: Num = Num::Exact(Rat::ONE);

    /// The number a float is: exact when it prints as a decimal a ratio can
    /// hold, which is every literal a file writes.
    pub(crate) fn of_f64(value: f64) -> Num {
        Rat::of_f64(value).map_or(Num::Float(value), Num::Exact)
    }

    pub(crate) fn value(self) -> f64 {
        match self {
            Num::Exact(rat) => rat.to_f64(),
            Num::Float(value) => value,
        }
    }

    #[allow(
        clippy::float_cmp,
        reason = "a coefficient is dropped only when it is exactly zero"
    )]
    pub(crate) fn is_zero(self) -> bool {
        match self {
            Num::Exact(rat) => rat.is_zero(),
            Num::Float(value) => value == 0.0,
        }
    }

    pub(crate) fn is_one(self) -> bool {
        self == Num::ONE
    }

    pub(crate) fn is_negative(self) -> bool {
        match self {
            Num::Exact(rat) => rat.is_negative(),
            Num::Float(value) => value < 0.0,
        }
    }

    pub(crate) fn neg(self) -> Num {
        match self {
            Num::Exact(rat) => Num::Exact(rat.neg()),
            Num::Float(value) => Num::Float(-value),
        }
    }

    pub(crate) fn abs(self) -> Num {
        if self.is_negative() { self.neg() } else { self }
    }

    pub(crate) fn add(self, other: Num) -> Num {
        self.combine(other, Rat::add, |a, b| a + b)
    }

    pub(crate) fn mul(self, other: Num) -> Num {
        self.combine(other, Rat::mul, |a, b| a * b)
    }

    /// The quotient; `None` for a divisor of zero.
    pub(crate) fn div(self, other: Num) -> Option<Num> {
        if other.is_zero() {
            return None;
        }
        Some(self.combine(other, Rat::div, |a, b| a / b))
    }

    /// The exact root of an exact square, else the float root; `None` below
    /// zero.
    pub(crate) fn sqrt(self) -> Option<Num> {
        if self.is_negative() {
            return None;
        }
        Some(match self {
            Num::Exact(rat) => rat
                .sqrt()
                .map_or(Num::Float(rat.to_f64().sqrt()), Num::Exact),
            Num::Float(value) => Num::Float(value.sqrt()),
        })
    }

    fn combine(
        self,
        other: Num,
        exact: fn(Rat, Rat) -> Option<Rat>,
        float: fn(f64, f64) -> f64,
    ) -> Num {
        if let (Num::Exact(a), Num::Exact(b)) = (self, other)
            && let Some(result) = exact(a, b)
        {
            return Num::Exact(result);
        }
        Num::Float(float(self.value(), other.value()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_arithmetic_stays_exact_and_a_float_is_contagious() {
        let sum = Num::of_f64(0.1).add(Num::of_f64(0.2));
        assert_eq!(sum, Num::of_f64(0.3));
        let mixed = Num::of_f64(0.5).mul(Num::Float(std::f64::consts::SQRT_2));
        assert!(matches!(mixed, Num::Float(_)));
        assert_eq!(Num::ONE.div(Num::of_f64(0.0)), None);
    }

    #[test]
    fn the_root_of_an_exact_square_is_exact() {
        assert_eq!(Num::of_f64(2.25).sqrt(), Some(Num::of_f64(1.5)));
        assert!(matches!(Num::of_f64(2.0).sqrt(), Some(Num::Float(_))));
        assert_eq!(Num::of_f64(-1.0).sqrt(), None);
    }
}
