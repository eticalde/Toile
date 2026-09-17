mod shape;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
pub use shape::BodyShape;

use crate::formula::Lookup;
use crate::{DocError, Origin};

/// The measurements a pattern can be resolved against, in centimetres.
///
/// The names are the user's data, not identifiers, which is why they are the
/// Spanish ones the mannequin tab already writes.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MeasureSet {
    /// The name the chooser shows.
    pub name: String,
    /// The measurements, by name, in centimetres.
    pub values: BTreeMap<String, f64>,
    /// What the body is generated from besides the tape, once someone set it.
    ///
    /// No formula reads it: a pattern resolves against the tape alone. A set
    /// without one writes no key for it, which is what keeps its file in
    /// format version 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phenotype: Option<BodyShape>,
    /// The library person this body was copied from, when it was.
    ///
    /// Under the phenotype's rule: a set without one writes no key for it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<Origin>,
}

impl MeasureSet {
    /// The measurements Toile knows how to autocomplete and check.
    ///
    /// A name outside the catalogue is allowed: the catalogue guides, it does
    /// not rule.
    pub const CATALOGUE: [&'static str; 20] = [
        "cintura",
        "cadera",
        "muslo",
        "rodilla",
        "tobillo",
        "tiro",
        "largo_lateral",
        "entrepierna",
        "altura_cadera",
        "estatura",
        "cuello",
        "pecho",
        "pecho_alto",
        "bajo_pecho",
        "hombros",
        "brazo",
        "brazo_contorno",
        "muneca",
        "largo_espalda",
        "cabeza",
    ];

    /// The catalogue's girths, top-down the body: the trunk from the neck to
    /// the ankle, then the arm and the head, which sit off its line.
    pub const GIRTHS: [&'static str; 12] = [
        "cuello",
        "pecho_alto",
        "pecho",
        "bajo_pecho",
        "cintura",
        "cadera",
        "muslo",
        "rodilla",
        "tobillo",
        "brazo_contorno",
        "muneca",
        "cabeza",
    ];

    /// The catalogue's lengths and its one width, top-down as well: the trunk,
    /// the arm, then the legs.
    pub const LENGTHS: [&'static str; 7] = [
        "largo_espalda",
        "brazo",
        "hombros",
        "tiro",
        "largo_lateral",
        "entrepierna",
        "altura_cadera",
    ];

    /// The catalogue's one whole-body measurement.
    pub const WHOLE: [&'static str; 1] = ["estatura"];

    /// Whether the catalogue names this measurement.
    pub fn is_catalogued(name: &str) -> bool {
        MeasureSet::CATALOGUE.contains(&name)
    }

    /// A measure set holding the pairs it is given, in centimetres.
    pub fn new<'a>(name: &str, values: impl IntoIterator<Item = (&'a str, f64)>) -> MeasureSet {
        MeasureSet {
            name: name.to_owned(),
            values: values
                .into_iter()
                .map(|(measure, value)| (measure.to_owned(), value))
                .collect(),
            phenotype: None,
            origin: None,
        }
    }

    /// The same set, with the body generated from `phenotype` as well.
    #[must_use]
    pub fn shaped(mut self, phenotype: BodyShape) -> MeasureSet {
        self.phenotype = Some(phenotype);
        self
    }

    /// The oldest format version whose reader keeps everything the set
    /// carries: a link needs 3, a phenotype 2, a bare tape 1.
    pub(crate) fn format_version(&self) -> u32 {
        if self.origin.is_some() {
            crate::json::VERSION_LINKED
        } else if self.phenotype.is_some() {
            crate::json::VERSION_EXTENDED
        } else {
            crate::json::VERSION
        }
    }

    /// The centimetres bound to `measure`, if the set carries it.
    pub fn get(&self, measure: &str) -> Option<f64> {
        self.values.get(measure).copied()
    }

    /// Whether the set carries `measure`.
    pub fn has(&self, measure: &str) -> bool {
        self.values.contains_key(measure)
    }

    /// The fingerprint of the tape and the shape this body is generated from.
    ///
    /// The very hash a library link is stamped with, so the two cannot come to
    /// disagree about when two bodies are the same body: the name, and where
    /// the values were copied from, are deliberately not in it. Anything
    /// derived from a body and kept between runs is filed under this.
    pub fn fingerprint(&self) -> String {
        crate::persona::fingerprint::of(&self.values, self.phenotype.as_ref())
    }

    /// Every name the set carries that the catalogue does not name.
    pub fn uncatalogued(&self) -> Vec<&str> {
        self.values
            .keys()
            .map(String::as_str)
            .filter(|name| !MeasureSet::is_catalogued(name))
            .collect()
    }
}

impl Lookup for MeasureSet {
    fn value(&self, name: &str) -> Option<f64> {
        self.get(name)
    }
}

/// Refuses a measurement JSON cannot spell, naming the first in key order.
///
/// The writer would spell it `null`, and the next open would refuse the file.
pub(crate) fn finite(values: &BTreeMap<String, f64>) -> Result<(), DocError> {
    values
        .iter()
        .find(|(_, value)| !value.is_finite())
        .map_or(Ok(()), |(name, _)| Err(DocError::NonFinite(name.clone())))
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "a measure set stores the centimetres it was given"
    )]

    use super::*;

    fn etienne() -> MeasureSet {
        MeasureSet::new("Etienne", [("cintura", 84.0), ("cadera", 98.0)])
    }

    #[test]
    fn a_measure_set_reads_the_names_it_carries() {
        let set = etienne();
        assert_eq!(set.get("cintura"), Some(84.0));
        assert_eq!(set.get("muslo"), None);
        assert!(set.has("cadera"));
        assert_eq!(set.value("cadera"), Some(98.0));
    }

    #[test]
    fn a_name_outside_the_catalogue_is_carried_and_reported() {
        let mut set = etienne();
        set.values.insert("largo_manga".to_owned(), 60.0);
        assert_eq!(set.get("largo_manga"), Some(60.0));
        assert_eq!(set.uncatalogued(), ["largo_manga"]);
        assert!(MeasureSet::is_catalogued("altura_cadera"));
        assert!(MeasureSet::is_catalogued("muneca"));
        assert!(!MeasureSet::is_catalogued("largo_manga"));
    }

    /// A name added to the catalogue and to no kind would never be offered
    /// where the kinds are listed, which is everywhere a person reads them.
    #[test]
    fn the_three_kinds_name_every_catalogue_measurement_once() {
        let mut kinds: Vec<&str> = MeasureSet::GIRTHS
            .iter()
            .chain(&MeasureSet::LENGTHS)
            .chain(&MeasureSet::WHOLE)
            .copied()
            .collect();
        let mut catalogue = MeasureSet::CATALOGUE.to_vec();
        kinds.sort_unstable();
        catalogue.sort_unstable();
        assert_eq!(kinds, catalogue);
    }

    #[test]
    fn an_empty_set_carries_nothing_and_says_so() {
        let set = MeasureSet::default();
        assert_eq!(set.get("cintura"), None);
        assert!(set.uncatalogued().is_empty());
        assert_eq!(set.format_version(), crate::json::VERSION);
    }

    #[test]
    fn a_shaped_set_keeps_its_tape_and_extends_the_format() {
        let set = etienne().shaped(BodyShape::default());
        assert_eq!(set.get("cintura"), Some(84.0));
        assert_eq!(set.phenotype.map(|shape| shape.age_years), Some(25.0));
        assert_eq!(set.format_version(), crate::json::VERSION_EXTENDED);
    }

    #[test]
    fn a_set_that_cannot_be_spelled_names_its_first_bad_measurement() {
        let mut set = etienne();
        assert_eq!(finite(&set.values), Ok(()));
        set.values.insert("muslo".to_owned(), f64::INFINITY);
        set.values.insert("brazo".to_owned(), f64::NAN);
        assert_eq!(
            finite(&set.values),
            Err(DocError::NonFinite("brazo".to_owned()))
        );
    }
}
