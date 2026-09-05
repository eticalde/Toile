use std::fmt;

use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Binding;
use crate::formula::Formula;

/// A binding is written as a number when it is one, and as its source text
/// when it is an expression.
///
/// An expression stays the string its author typed, spacing and all: it is
/// more diffable than the number it stands for, not less, and it is the half
/// of the file a reader can reason about.
impl Serialize for Binding {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Binding::Literal(value) => serializer.serialize_f64(*value),
            Binding::Formula(formula) => serializer.serialize_str(formula.source()),
        }
    }
}

impl<'de> Deserialize<'de> for Binding {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Binding, D::Error> {
        deserializer.deserialize_any(Written)
    }
}

struct Written;

impl Visitor<'_> for Written {
    type Value = Binding;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a number, or a formula written as text")
    }

    fn visit_f64<E: Error>(self, value: f64) -> Result<Binding, E> {
        Ok(Binding::Literal(value))
    }

    fn visit_i64<E: Error>(self, value: i64) -> Result<Binding, E> {
        Ok(Binding::Literal(value as f64))
    }

    fn visit_u64<E: Error>(self, value: u64) -> Result<Binding, E> {
        Ok(Binding::Literal(value as f64))
    }

    /// Source that spells a plain number stays a formula here, unlike the
    /// binding a user types: a file has to read back as the document that
    /// wrote it, down to which of the two a coordinate was.
    fn visit_str<E: Error>(self, source: &str) -> Result<Binding, E> {
        match Formula::parse(source) {
            Ok(formula) => Ok(Binding::Formula(formula)),
            Err(error) => Err(E::custom(format!("`{source}` is not a formula: {error}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp, reason = "a literal reads back the number it wrote")]

    use super::*;

    fn round_trip(binding: &Binding) -> Binding {
        let written = serde_json::to_string(binding).expect("a binding writes");
        serde_json::from_str(&written).expect("what was written reads")
    }

    #[test]
    fn a_literal_is_written_as_a_number() {
        assert_eq!(
            serde_json::to_string(&Binding::literal(22.0)).expect("a binding writes"),
            "22.0"
        );
        assert_eq!(
            round_trip(&Binding::literal(20.875)),
            Binding::literal(20.875)
        );
    }

    #[test]
    fn a_formula_is_written_as_its_source() {
        let binding = Binding::parse("cintura / 4 + 1").expect("the source parses");
        let written = serde_json::to_string(&binding).expect("a binding writes");
        assert_eq!(written, "\"cintura / 4 + 1\"");
        assert_eq!(round_trip(&binding), binding);
    }

    #[test]
    fn a_formula_that_spells_a_number_stays_a_formula() {
        let formula = Formula::parse("22").expect("the source parses");
        let binding = Binding::Formula(formula);
        assert_eq!(round_trip(&binding), binding);
    }

    #[test]
    fn a_whole_number_read_as_an_integer_is_still_a_literal() {
        let binding: Binding = serde_json::from_str("104").expect("an integer reads");
        assert_eq!(binding, Binding::Literal(104.0));
        let signed: Binding = serde_json::from_str("-6").expect("an integer reads");
        assert_eq!(signed, Binding::Literal(-6.0));
    }

    #[test]
    fn source_that_is_not_a_formula_says_where_it_stops() {
        let error = serde_json::from_str::<Binding>("\"cintura / \"")
            .expect_err("the source does not parse")
            .to_string();
        assert!(error.contains("is not a formula"), "{error}");
        assert!(error.contains("byte"), "{error}");
    }
}
