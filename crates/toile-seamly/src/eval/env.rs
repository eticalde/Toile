use std::collections::BTreeMap;

use crate::{Formula, FormulaError, Measurements};

/// The name a length formula of an along-line point uses for the distance
/// between its two points.
pub(crate) const CURRENT_LENGTH: &str = "CurrentLength";
/// `Line_a_b`: the distance between two points a segment joins.
pub(crate) const LINE: &str = "Line_";
/// `AngleLine_a_b`: the heading from `a` to `b` of a segment joining them.
pub(crate) const ANGLE_LINE: &str = "AngleLine_";
/// `Spl_a_b`: the length of the spline from `a` to `b`.
pub(crate) const SPLINE: &str = "Spl_";

/// Two drawings that give one name values further apart than this define
/// two things: the same segment met twice lands within rounding of itself.
const SAME_VALUE: f64 = 1e-9;

/// Every name a formula can cite, filled in file order.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Env {
    measures: BTreeMap<String, f64>,
    variables: BTreeMap<String, f64>,
    /// `Line_`, `AngleLine_` and `Spl_` values; `None` marks a name two
    /// drawings define differently, which only matters if a formula cites it.
    drawn: BTreeMap<String, Option<f64>>,
}

impl Env {
    pub(crate) fn new(measurements: &Measurements) -> Self {
        Self {
            measures: measurements
                .entries()
                .iter()
                .map(|m| (m.name.clone(), m.value))
                .collect(),
            ..Self::default()
        }
    }

    /// The value of `name`, with `current` standing for `CurrentLength` where
    /// the formula has one.
    pub(crate) fn value(&self, name: &str, current: Option<f64>) -> Result<f64, FormulaError> {
        if let (CURRENT_LENGTH, Some(length)) = (name, current) {
            return Ok(length);
        }
        if let Some(drawn) = self.drawn.get(name) {
            return drawn.ok_or_else(|| FormulaError::Ambiguous(name.to_owned()));
        }
        self.variables
            .get(name)
            .or_else(|| self.measures.get(name))
            .copied()
            .ok_or_else(|| FormulaError::UnknownName(name.to_owned()))
    }

    pub(crate) fn eval(
        &self,
        formula: &Formula,
        current: Option<f64>,
    ) -> Result<f64, FormulaError> {
        formula.eval(&|name| self.value(name, current))
    }

    /// Defines a pattern variable; `false` when it would hide a measurement.
    pub(crate) fn set_variable(&mut self, name: &str, value: f64) -> bool {
        if self.measures.contains_key(name) {
            return false;
        }
        self.variables.insert(name.to_owned(), value);
        true
    }

    pub(crate) fn variable(&self, name: &str) -> Option<f64> {
        self.variables.get(name).copied()
    }

    pub(crate) fn drawn(&self, name: &str) -> Option<f64> {
        self.drawn.get(name).copied().flatten()
    }

    /// Defines a value a drawing gives a name. The later definition wins, as
    /// file order would have it; one far from the earlier makes the name
    /// ambiguous.
    pub(crate) fn define(&mut self, name: String, value: f64) {
        let entry = match self.drawn.get(&name) {
            Some(None) => None,
            Some(Some(old)) => {
                let mut apart = (old - value).abs();
                if name.starts_with(ANGLE_LINE) {
                    apart = apart.min((360.0 - apart).abs());
                }
                (apart <= SAME_VALUE).then_some(value)
            }
            None => Some(value),
        };
        self.drawn.insert(name, entry);
    }
}
