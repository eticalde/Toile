/// A rational number held exactly, as a ratio of two integers.
///
/// Every number a Seamly file writes is a decimal, and a coordinate unrolled
/// through a chain of constructions adds a handful of them together. In binary
/// floating point that sum picks up digits the file never wrote; as a ratio it
/// stays the decimal a person would have typed. Arithmetic that would overflow
/// says so with `None`, and the caller falls back to floating point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Rat {
    num: i128,
    /// Always positive, and coprime with `num`.
    den: i128,
}

impl Rat {
    pub(crate) const ZERO: Rat = Rat { num: 0, den: 1 };
    pub(crate) const ONE: Rat = Rat { num: 1, den: 1 };

    pub(crate) fn int(value: i64) -> Rat {
        Rat {
            num: i128::from(value),
            den: 1,
        }
    }

    pub(crate) fn new(num: i128, den: i128) -> Option<Rat> {
        if den == 0 {
            return None;
        }
        let sign = if den < 0 { -1 } else { 1 };
        let divisor = gcd(num.checked_abs()?, den.checked_abs()?).max(1);
        Some(Rat {
            num: sign * (num / divisor),
            den: sign * (den / divisor),
        })
    }

    /// The decimal a float prints as, read back exactly.
    ///
    /// Rust prints a float as the shortest decimal that reads back to it, so a
    /// literal parsed from a file prints as the file wrote it and becomes that
    /// decimal here, not the binary fraction nearest to it.
    pub(crate) fn of_f64(value: f64) -> Option<Rat> {
        if !value.is_finite() {
            return None;
        }
        Rat::parse_decimal(&format!("{value}"))
    }

    fn parse_decimal(text: &str) -> Option<Rat> {
        let (negative, digits) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
        let mut num: i128 = 0;
        for byte in whole.bytes().chain(fraction.bytes()) {
            if !byte.is_ascii_digit() {
                return None;
            }
            num = num.checked_mul(10)?.checked_add(i128::from(byte - b'0'))?;
        }
        let den = 10_i128.checked_pow(u32::try_from(fraction.len()).ok()?)?;
        Rat::new(if negative { -num } else { num }, den)
    }

    pub(crate) fn is_zero(self) -> bool {
        self.num == 0
    }

    pub(crate) fn is_negative(self) -> bool {
        self.num < 0
    }

    pub(crate) fn neg(self) -> Rat {
        Rat {
            num: -self.num,
            den: self.den,
        }
    }

    pub(crate) fn abs(self) -> Rat {
        Rat {
            num: self.num.abs(),
            den: self.den,
        }
    }

    pub(crate) fn add(self, other: Rat) -> Option<Rat> {
        let num = self
            .num
            .checked_mul(other.den)?
            .checked_add(other.num.checked_mul(self.den)?)?;
        Rat::new(num, self.den.checked_mul(other.den)?)
    }

    pub(crate) fn mul(self, other: Rat) -> Option<Rat> {
        Rat::new(
            self.num.checked_mul(other.num)?,
            self.den.checked_mul(other.den)?,
        )
    }

    pub(crate) fn div(self, other: Rat) -> Option<Rat> {
        Rat::new(
            self.num.checked_mul(other.den)?,
            self.den.checked_mul(other.num)?,
        )
    }

    /// The whole part of the quotient by `other`, rounded toward minus
    /// infinity; `None` for a divisor of zero.
    pub(crate) fn floor_div(self, other: Rat) -> Option<i128> {
        let quotient = self.div(other)?;
        Some(quotient.num.div_euclid(quotient.den))
    }

    /// The exact square root, when there is one.
    pub(crate) fn sqrt(self) -> Option<Rat> {
        Some(Rat {
            num: exact_root(self.num)?,
            den: exact_root(self.den)?,
        })
    }

    /// The numerator, when the number is `1 / n` for a whole `n` above one.
    pub(crate) fn reciprocal_of_whole(self) -> Option<i128> {
        (self.num == 1 && self.den > 1).then_some(self.den)
    }

    /// The numerator and the denominator.
    pub(crate) fn parts(self) -> (i128, i128) {
        (self.num, self.den)
    }

    /// The number as a decimal, when it has a finite one.
    pub(crate) fn decimal(self) -> Option<String> {
        let (mut twos, mut fives, mut rest) = (0_u32, 0_u32, self.den);
        while rest % 2 == 0 {
            rest /= 2;
            twos += 1;
        }
        while rest % 5 == 0 {
            rest /= 5;
            fives += 1;
        }
        if rest != 1 {
            return None;
        }
        let places = twos.max(fives);
        let scale = 10_i128.checked_pow(places)? / self.den;
        let scaled = self.num.checked_abs()?.checked_mul(scale)?;
        let unit = 10_i128.checked_pow(places)?;
        let (whole, fraction) = (scaled / unit, scaled % unit);
        let sign = if self.is_negative() { "-" } else { "" };
        if places == 0 {
            return Some(format!("{sign}{whole}"));
        }
        let digits = format!("{fraction:0width$}", width = places as usize);
        let digits = digits.trim_end_matches('0');
        if digits.is_empty() {
            Some(format!("{sign}{whole}"))
        } else {
            Some(format!("{sign}{whole}.{digits}"))
        }
    }

    /// The nearest float: the decimal read by the float parser when there is
    /// one, so that it rounds once and correctly.
    pub(crate) fn to_f64(self) -> f64 {
        self.decimal()
            .and_then(|text| text.parse().ok())
            .unwrap_or(self.num as f64 / self.den as f64)
    }
}

fn gcd(mut a: i128, mut b: i128) -> i128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// The whole square root of `value`, when it is a perfect square.
fn exact_root(value: i128) -> Option<i128> {
    if value < 0 {
        return None;
    }
    // A float's root lands within one of the integer root for every value a
    // pattern's decimals produce, so a step either side finds it.
    let guess = (value as f64).sqrt() as i128;
    (guess.saturating_sub(1)..=guess.saturating_add(1))
        .find(|&root| root >= 0 && root.checked_mul(root) == Some(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rat(text: &str) -> Rat {
        Rat::parse_decimal(text).expect("a decimal")
    }

    #[test]
    fn a_decimal_the_file_writes_is_held_as_that_decimal() {
        assert_eq!(Rat::of_f64(0.79375), Some(rat("0.79375")));
        assert_eq!(rat("0.79375").decimal().as_deref(), Some("0.79375"));
        assert_eq!(
            Rat::of_f64(-12.3456).and_then(Rat::decimal).as_deref(),
            Some("-12.3456")
        );
    }

    #[test]
    fn sums_of_decimals_stay_the_decimal_a_person_would_write() {
        let sum = rat("0.79375").add(rat("29.9829")).expect("no overflow");
        assert_eq!(sum.decimal().as_deref(), Some("30.77665"));
        let product = rat("3").mul(rat("1.2")).expect("no overflow");
        assert_eq!(product.decimal().as_deref(), Some("3.6"));
        assert_eq!(
            rat("7").mul(rat("6.5")).and_then(Rat::decimal).as_deref(),
            Some("45.5")
        );
    }

    #[test]
    fn a_third_has_no_decimal_and_says_so() {
        let third = Rat::ONE.div(Rat::int(3)).expect("no overflow");
        assert_eq!(third.decimal(), None);
        assert_eq!(third.reciprocal_of_whole(), Some(3));
        assert_eq!(Rat::ONE.div(Rat::ZERO), None);
    }

    #[test]
    fn a_perfect_square_has_an_exact_root_and_nothing_else_does() {
        assert_eq!(rat("2.25").sqrt(), Some(rat("1.5")));
        assert_eq!(rat("2").sqrt(), None);
        assert_eq!(rat("-4").sqrt(), None);
    }

    #[test]
    fn floor_division_rounds_toward_minus_infinity() {
        assert_eq!(rat("450").floor_div(rat("360")), Some(1));
        assert_eq!(rat("-90").floor_div(rat("360")), Some(-1));
    }
}
