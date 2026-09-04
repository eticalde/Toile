use toile_doc::formula::{Dependency, EvalError};
use toile_doc::{Doc, VariableKey};

/// The pattern variables, each after the ones it reads.
///
/// A name no variable defines is a measurement, and measurements are bound
/// before any variable is, so reading one places no edge. The queue starts in
/// key order, which makes the result canonical rather than merely correct.
///
/// # Errors
/// `EvalError::Cycle`, naming the loop, when variables depend on each other.
pub fn evaluation_order(doc: &Doc) -> Result<Vec<VariableKey>, EvalError> {
    let keys: Vec<VariableKey> = doc.variables.keys().collect();
    let graph: Vec<Dependency<'_>> = doc
        .variables
        .iter()
        .map(|(_, variable)| Dependency {
            name: &variable.name,
            reads: variable.value.names(),
        })
        .collect();
    Ok(toile_doc::formula::evaluation_order(&graph)?
        .into_iter()
        .map(|index| keys[index])
        .collect())
}

#[cfg(test)]
mod tests {
    use toile_doc::{Binding, MeasureSet, Variable};

    use super::*;

    fn doc(pairs: &[(&str, &str)]) -> Doc {
        let mut doc = Doc::new(MeasureSet::new("Etienne", [("cadera", 98.0)]));
        for &(name, source) in pairs {
            let value = Binding::parse(source).expect("the test writes its own sources");
            doc.variables.insert(Variable::new(name, value));
        }
        doc
    }

    fn names(doc: &Doc, order: &[VariableKey]) -> Vec<String> {
        order
            .iter()
            .map(|&key| {
                doc.variables
                    .get(key)
                    .expect("the order names live keys")
                    .name
                    .clone()
            })
            .collect()
    }

    /// The two seam tolerances a new document seeds, which depend on nothing.
    fn seeded() -> Vec<String> {
        vec![
            "tolerancia_costura".to_owned(),
            "tolerancia_ratio".to_owned(),
        ]
    }

    #[test]
    fn a_variable_may_reference_one_declared_later() {
        let doc = doc(&[
            ("raya", "cadera / 4 - extension_tiro"),
            ("extension_tiro", "cadera / 16"),
        ]);
        let order = evaluation_order(&doc).expect("the graph has no cycle");
        let mut expected = seeded();
        expected.extend(["extension_tiro".to_owned(), "raya".to_owned()]);
        assert_eq!(names(&doc, &order), expected);
    }

    #[test]
    fn a_measurement_places_no_edge() {
        let doc = doc(&[("holgura", "cadera / 100")]);
        let order = evaluation_order(&doc).expect("a measurement closes no loop");
        assert_eq!(order.len(), doc.variables.len());
    }

    #[test]
    fn a_cycle_is_reported_by_name() {
        let doc = doc(&[("a", "b + 1"), ("b", "a + 1")]);
        assert_eq!(
            evaluation_order(&doc),
            Err(EvalError::Cycle("a -> b -> a".to_owned()))
        );
    }

    #[test]
    fn a_document_with_no_variables_of_its_own_still_orders() {
        let doc = doc(&[]);
        let order = evaluation_order(&doc).expect("the graph has no cycle");
        assert_eq!(names(&doc, &order), seeded());
    }
}
