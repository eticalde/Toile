use std::collections::{BTreeMap, BTreeSet};

use crate::{Error, Formula, Measurements, Pattern, mapping};

/// The names a Toile formula keeps for its functions, which no variable can
/// take.
const FUNCTIONS: [&str; 4] = ["min", "max", "abs", "sqrt"];

/// A measurement of the imported body, under the name the product gives it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Measure {
    /// The Toile name: the catalogue's when the mapping file names one.
    pub(crate) toile: Option<String>,
    /// Whether that name came from the mapping file.
    pub(crate) mapped: bool,
    /// Centimetres.
    pub(crate) value: f64,
}

/// Every Seamly name a Toile formula can read, respelled.
///
/// A measurement goes by its catalogue name where the mapping file gives one
/// and by its own name where it does not; a pattern variable drops its `#`
/// and its capitals, so `#PhL` reads `phl`.
#[derive(Debug, Clone)]
pub(crate) struct Names {
    variables: BTreeMap<String, String>,
    measurements: BTreeMap<String, Measure>,
    /// The Seamly measurements some formula of the product reads.
    read: BTreeSet<String>,
}

impl Names {
    pub(crate) fn new(pattern: &Pattern, body: &Measurements) -> Result<Names, Error> {
        let mut measurements = BTreeMap::new();
        for entry in body.entries() {
            let mapped = mapping::toile_measurement(&entry.name);
            let measure = Measure {
                toile: mapped.map(str::to_owned).or_else(|| ident(&entry.name)),
                mapped: mapped.is_some(),
                value: entry.value,
            };
            measurements.insert(entry.name.clone(), measure);
        }
        let taken: BTreeSet<&str> = measurements
            .values()
            .filter_map(|m| m.toile.as_deref())
            .collect();
        let mut variables = BTreeMap::new();
        let mut spelled = BTreeSet::new();
        for variable in &pattern.variables {
            let toile = ident(&variable.name).ok_or_else(|| {
                Error::Product(format!(
                    "the variable `{}` has no spelling a Toile formula can read",
                    variable.name
                ))
            })?;
            if taken.contains(toile.as_str()) || !spelled.insert(toile.clone()) {
                return Err(Error::Product(format!(
                    "the variable `{}` would be called `{toile}`, which another name already is",
                    variable.name
                )));
            }
            variables.insert(variable.name.clone(), toile);
        }
        Ok(Names {
            variables,
            measurements,
            read: BTreeSet::new(),
        })
    }

    /// The Toile name of the pattern variable `seamly`, `#` included.
    pub(crate) fn variable(&self, seamly: &str) -> Option<&str> {
        self.variables.get(seamly).map(String::as_str)
    }

    /// The Toile name of the measurement `seamly`, which from now on the body
    /// must carry.
    pub(crate) fn measurement(&mut self, seamly: &str) -> Result<String, Error> {
        let measure = self.measurements.get(seamly).ok_or_else(|| {
            Error::Product(format!("the body has no measurement called `{seamly}`"))
        })?;
        let toile = measure.toile.clone().ok_or_else(|| {
            Error::Product(format!(
                "the measurement `{seamly}` has no catalogue name and no spelling a \
                 Toile formula can read"
            ))
        })?;
        self.read.insert(seamly.to_owned());
        Ok(toile)
    }

    /// Whether some pattern variable is spelled `toile` in Toile.
    pub(crate) fn variable_spelled(&self, toile: &str) -> bool {
        self.variables.values().any(|spelled| spelled == toile)
    }

    /// Whether some measurement of the body goes by `toile` in Toile.
    pub(crate) fn measurement_spelled(&self, toile: &str) -> bool {
        self.measurements
            .values()
            .any(|measure| measure.toile.as_deref() == Some(toile))
    }

    /// Every measurement of the body, by its Seamly name, in name order.
    pub(crate) fn measurements(&self) -> &BTreeMap<String, Measure> {
        &self.measurements
    }

    /// Whether some formula of the product reads the measurement `seamly`.
    pub(crate) fn is_read(&self, seamly: &str) -> bool {
        self.read.contains(seamly)
    }

    /// A variable's formula respelled for Toile, the text otherwise kept as
    /// its author wrote it: `(#Tcw-#Ftcw)` becomes `(tcw-ftcw)`, and a
    /// measurement takes the name [`Names::measurement`] gives it.
    pub(crate) fn respell(&mut self, formula: &Formula) -> Result<String, Error> {
        let source = formula.source();
        let mut out = String::with_capacity(source.len());
        let mut rest = source;
        while let Some(c) = rest.chars().next() {
            let run = if c.is_ascii_digit() || c == '.' {
                rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '.'))
            } else if c == '#' || c == '@' || c == '_' || c.is_alphabetic() {
                rest.char_indices()
                    .skip(1)
                    .find(|&(_, c)| !(c == '_' || c.is_alphanumeric()))
                    .map(|(at, _)| at)
            } else {
                Some(c.len_utf8())
            };
            let (token, after) = rest.split_at(run.unwrap_or(rest.len()));
            if c == '#' {
                let toile = self.variable(token).ok_or_else(|| {
                    Error::Product(format!("`{source}` cites `{token}`, which is no variable"))
                })?;
                out.push_str(toile);
            } else if c == '@' || c == '_' || c.is_alphabetic() {
                out.push_str(&self.measurement(token)?);
            } else {
                out.push_str(token);
            }
            rest = after;
        }
        Ok(out)
    }
}

/// A Seamly name as a Toile identifier: its `#` or `@` dropped, lower case,
/// and nothing a Toile name cannot hold.
///
/// `None` for a name with a character outside ASCII letters, digits and
/// underscores, one that starts with a digit, or one a function already has.
pub(crate) fn ident(name: &str) -> Option<String> {
    let bare = name.strip_prefix(['#', '@']).unwrap_or(name);
    let lower = bare.to_ascii_lowercase();
    let mut chars = lower.chars();
    let first = chars.next()?;
    let valid = (first.is_ascii_lowercase() || first == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !FUNCTIONS.contains(&lower.as_str());
    valid.then_some(lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seamly_name_loses_its_sigil_and_its_capitals() {
        assert_eq!(ident("#PhL").as_deref(), Some("phl"));
        assert_eq!(ident("#CordD").as_deref(), Some("cordd"));
        assert_eq!(ident("B_hip_start").as_deref(), Some("b_hip_start"));
        assert_eq!(
            ident("Line_b_knee_end_B1").as_deref(),
            Some("line_b_knee_end_b1")
        );
    }

    #[test]
    fn a_name_no_toile_formula_could_read_has_no_spelling() {
        assert_eq!(ident("#1st"), None);
        assert_eq!(ident("#caída"), None);
        assert_eq!(ident("#Sqrt"), None);
        assert_eq!(ident("#"), None);
    }
}
