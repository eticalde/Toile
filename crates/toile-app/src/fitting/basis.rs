use std::collections::BTreeMap;

use toile_engine::draft::{BodyShape, Doc, MeasureSet};
use toile_engine::session::Session;

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

/// The body the product on the table resolves against, or the loose body while
/// no product is open.
pub fn tape(session: &Session, loose: &MeasureSet) -> MeasureSet {
    tape_of(session.draft().map(toile_engine::draft::Draft::doc), loose)
}

/// The body `doc` resolves against, or `loose` where there is no document.
///
/// The loose body is the mannequin tab's own, handed in rather than copied:
/// with no product open every slider in that tab writes into it, and the body
/// the cloth falls on has to be that very one. Reading a second reference tape
/// here is what let the two come to disagree about which body it is.
pub fn tape_of(doc: Option<&Doc>, loose: &MeasureSet) -> MeasureSet {
    doc.and_then(|doc| doc.measures().cloned())
        .unwrap_or_else(|| loose.clone())
}
