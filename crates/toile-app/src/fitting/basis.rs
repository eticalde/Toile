use std::collections::BTreeMap;

use toile_engine::draft::{BodyShape, MeasureSet};

/// Every input a body is solved from, and nothing else, so a rename costs no
/// bake.
#[derive(Debug, Clone, PartialEq)]
pub struct Basis {
    values: BTreeMap<String, f64>,
    shape: BodyShape,
}

impl Basis {
    /// What the body a tape asks for is solved from.
    pub fn of(tape: &MeasureSet) -> Basis {
        Basis {
            values: tape.values.clone(),
            shape: tape.phenotype.unwrap_or_default(),
        }
    }
}
