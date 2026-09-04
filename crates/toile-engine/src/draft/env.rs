use std::collections::BTreeMap;

use thiserror::Error;
use toile_doc::Doc;
use toile_doc::formula::{EvalError, Lookup};

use super::order;

/// What every name in the document's formulas is worth, in centimetres.
///
/// Flat by design: a measurement and a pattern variable are the same kind of
/// name to a formula, which is what lets an ease be written wherever a
/// measurement can.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Env {
    values: BTreeMap<String, f64>,
}

/// What stops a document from having an environment at all.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EnvError {
    /// The body the document resolves against is not in it.
    #[error("the document resolves against a body it does not carry")]
    NoMannequin,
    /// Pattern variables that cannot be put in an order.
    #[error("the pattern variables cannot be ordered: {0}")]
    Order(EvalError),
    /// A variable named after a measurement.
    ///
    /// Silently letting one win over the other is how a pattern comes to mean
    /// something other than what it reads.
    #[error("the variable `{0}` is named after a measurement")]
    Shadowed(String),
    /// A variable whose binding does not resolve.
    #[error("the variable `{name}` does not resolve: {error}")]
    Variable {
        /// The variable that failed.
        name: String,
        /// Why it failed.
        error: EvalError,
    },
}

impl Env {
    /// What `name` is worth, in centimetres.
    pub fn value(&self, name: &str) -> Option<f64> {
        self.values.get(name).copied()
    }
}

impl Lookup for Env {
    fn value(&self, name: &str) -> Option<f64> {
        Env::value(self, name)
    }
}

/// Builds the environment: the chosen body's measurements, then the pattern
/// variables in the order they read each other.
///
/// # Errors
/// `EnvError` for a missing body, a variable that shadows a measurement, or a
/// variable whose binding does not resolve — a cycle included.
pub fn build(doc: &Doc) -> Result<Env, EnvError> {
    let measures = doc.measures().ok_or(EnvError::NoMannequin)?;
    let mut env = Env {
        values: measures.values.clone(),
    };
    for (_, variable) in doc.variables.iter() {
        if measures.has(&variable.name) {
            return Err(EnvError::Shadowed(variable.name.clone()));
        }
    }
    let order = order::evaluation_order(doc).map_err(EnvError::Order)?;
    for key in order {
        let variable = doc
            .variables
            .get(key)
            .expect("the order names keys the document just handed out");
        let value = variable
            .value
            .eval(&env)
            .map_err(|error| EnvError::Variable {
                name: variable.name.clone(),
                error,
            })?;
        env.values.insert(variable.name.clone(), value);
    }
    Ok(env)
}

#[cfg(test)]
mod tests {
    use toile_doc::{Binding, MeasureSet, Variable};

    use super::*;

    fn doc(pairs: &[(&str, &str)]) -> Doc {
        let mut doc = Doc::new(MeasureSet::new(
            "Etienne",
            [("cadera", 98.0), ("cintura", 84.0)],
        ));
        for &(name, source) in pairs {
            let value = Binding::parse(source).expect("the test writes its own sources");
            doc.variables.insert(Variable::new(name, value));
        }
        doc
    }

    #[test]
    fn the_measurements_of_the_chosen_body_are_bound_first() {
        let env = build(&doc(&[])).expect("the document has a body");
        assert_eq!(env.value("cadera"), Some(98.0));
        assert_eq!(env.value("largo_lateral"), None);
    }

    #[test]
    fn a_variable_reads_the_ones_it_depends_on_whatever_the_order() {
        let env = build(&doc(&[
            ("raya", "(cadera / 4 + holgura - extension_tiro) / 2"),
            ("holgura", "1"),
            ("extension_tiro", "cadera / 16"),
        ]))
        .expect("the graph has no cycle");
        assert_eq!(env.value("extension_tiro"), Some(98.0 / 16.0));
        assert_eq!(
            env.value("raya"),
            Some((98.0 / 4.0 + 1.0 - 98.0 / 16.0) / 2.0)
        );
    }

    #[test]
    fn a_variable_shadowing_a_measure_is_rejected() {
        assert_eq!(
            build(&doc(&[("cintura", "70")])),
            Err(EnvError::Shadowed("cintura".to_owned()))
        );
    }

    #[test]
    fn a_cycle_stops_the_environment() {
        assert_eq!(
            build(&doc(&[("a", "b + 1"), ("b", "a + 1")])),
            Err(EnvError::Order(EvalError::Cycle("a -> b -> a".to_owned())))
        );
    }

    #[test]
    fn an_unknown_name_names_the_variable_that_reads_it() {
        assert_eq!(
            build(&doc(&[("raya", "muslo / 2")])),
            Err(EnvError::Variable {
                name: "raya".to_owned(),
                error: EvalError::UnknownName("muslo".to_owned()),
            })
        );
    }

    #[test]
    fn a_document_without_a_body_has_no_environment() {
        let mut doc = doc(&[]);
        doc.mannequins
            .remove(doc.resolve_with)
            .expect("the document was built with one body");
        assert_eq!(build(&doc), Err(EnvError::NoMannequin));
    }
}
