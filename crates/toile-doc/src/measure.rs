use std::collections::BTreeMap;

use crate::formula::Lookup;

/// The measurements a pattern can be resolved against, in centimetres.
///
/// The names are the user's data, not identifiers, which is why they are the
/// Spanish ones the mannequin tab already writes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeasureSet {
    /// The name the chooser shows.
    pub name: String,
    /// The measurements, by name, in centimetres.
    pub values: BTreeMap<String, f64>,
}

impl MeasureSet {
    /// The measurements Toile knows how to autocomplete and check.
    ///
    /// A name outside the catalogue is allowed: the catalogue guides, it does
    /// not rule.
    pub const CATALOGUE: [&'static str; 10] = [
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
    ];

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
        assert!(!MeasureSet::is_catalogued("largo_manga"));
    }

    #[test]
    fn an_empty_set_carries_nothing_and_says_so() {
        let set = MeasureSet::default();
        assert_eq!(set.get("cintura"), None);
        assert!(set.uncatalogued().is_empty());
    }
}
