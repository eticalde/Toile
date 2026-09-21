/// The mapping itself: a data file beside the crate, never names in the code.
const TABLE: &str = include_str!("../data/measurements.tsv");

/// Every pair the mapping file holds, Seamly name first, in file order.
///
/// # Panics
/// If a line of the file is not two names split by a tab; a test reads the
/// whole file, so a malformed line fails the build's tests rather than an
/// import.
pub fn measurement_pairs() -> impl Iterator<Item = (&'static str, &'static str)> {
    TABLE
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            line.split_once('\t')
                .expect("every mapping line is two names split by a tab")
        })
}

/// The Toile catalogue name a Seamly measurement stands for, if it stands for
/// one.
pub fn toile_measurement(seamly: &str) -> Option<&'static str> {
    measurement_pairs()
        .find(|&(from, _)| from == seamly)
        .map(|(_, to)| to)
}

/// The Seamly measurement a Toile catalogue name stands for, if the mapping
/// names one.
pub fn seamly_measurement(toile: &str) -> Option<&'static str> {
    measurement_pairs()
        .find(|&(_, to)| to == toile)
        .map(|(from, _)| from)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use toile_doc::MeasureSet;

    use super::*;

    #[test]
    fn every_line_maps_one_name_to_one_catalogue_name_and_each_side_once() {
        let pairs: Vec<_> = measurement_pairs().collect();
        assert!(!pairs.is_empty());
        let seamly: BTreeSet<_> = pairs.iter().map(|&(from, _)| from).collect();
        let toile: BTreeSet<_> = pairs.iter().map(|&(_, to)| to).collect();
        assert_eq!(seamly.len(), pairs.len(), "a Seamly name is mapped twice");
        assert_eq!(toile.len(), pairs.len(), "a Toile name is mapped twice");
        for (from, to) in pairs {
            assert!(MeasureSet::is_catalogued(to), "{from} maps to {to}");
            assert!(!from.contains(char::is_whitespace), "{from:?}");
        }
    }

    #[test]
    fn a_name_is_looked_up_both_ways() {
        for (from, to) in measurement_pairs() {
            assert_eq!(toile_measurement(from), Some(to));
            assert_eq!(seamly_measurement(to), Some(from));
        }
        assert_eq!(toile_measurement("cadera"), None);
        assert_eq!(seamly_measurement("muslo"), None);
    }
}
