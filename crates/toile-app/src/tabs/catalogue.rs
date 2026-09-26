use toile_engine::draft::MeasureSet;

/// Every measurement of the catalogue with the name a panel shows for it.
///
/// The catalogue's own names are what a formula types, so they carry no accent
/// and no space; these are the same measurements as a person reads them. One
/// table and not one per panel: the mannequin tab and the drafting table name
/// the same ring, and a ring named two ways is two rings to whoever reads it.
///
/// In the catalogue's own order, so that the two lists can be read side by
/// side, and as long as it, so that a measurement added there does not compile
/// until it is named here.
const SHOWN: [(&str, &str); MeasureSet::CATALOGUE.len()] = [
    ("cintura", "Cintura"),
    ("cadera", "Cadera"),
    ("muslo", "Muslo"),
    ("rodilla", "Rodilla"),
    ("tobillo", "Tobillo"),
    ("tiro", "Tiro"),
    ("largo_lateral", "Largo lateral"),
    ("entrepierna", "Entrepierna"),
    ("altura_cadera", "Altura de cadera"),
    ("estatura", "Estatura"),
    ("cuello", "Cuello"),
    ("pecho", "Contorno de pecho"),
    ("pecho_alto", "Pecho alto"),
    ("bajo_pecho", "Bajo pecho"),
    ("hombros", "Ancho de hombros"),
    ("brazo", "Largo de brazo"),
    ("brazo_contorno", "Contorno de brazo"),
    ("muneca", "Muñeca"),
    ("largo_espalda", "Largo de espalda"),
    ("cabeza", "Contorno de cabeza"),
];

/// The catalogue in the three kinds a panel lists it under, each under the
/// heading both panels give it.
///
/// The names inside each kind are the document's own arrays, so the order a
/// person reads them down a panel is the order the catalogue carries, and there
/// is no second ordering to keep in step with it.
pub const KINDS: [(&str, &[&str]); 3] = [
    ("Contornos", &MeasureSet::GIRTHS),
    ("Largos y anchos", &MeasureSet::LENGTHS),
    ("Cuerpo", &MeasureSet::WHOLE),
];

/// What a panel calls `name`, which is `name` itself for one the catalogue does
/// not carry.
///
/// A body's own measurement falls through untouched: whoever took it typed the
/// name they wanted, and dressing it up would show them a word they never
/// wrote.
pub fn label(name: &str) -> &str {
    SHOWN
        .into_iter()
        .find_map(|(catalogued, shown)| (catalogued == name).then_some(shown))
        .unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every catalogued measurement is shown under a name of its own.
    ///
    /// The length of the table is pinned by the catalogue, so what this adds is
    /// that the twenty entries are the twenty names and not twenty of anything:
    /// a name misspelled here would be shown as the identifier it failed to
    /// match, in the middle of a panel written in Spanish.
    #[test]
    fn every_catalogued_measurement_is_shown_under_its_own_name() {
        let mut shown: Vec<&str> = Vec::new();
        for name in MeasureSet::CATALOGUE {
            let label = label(name);
            assert_ne!(label, name, "{name} falls through the table");
            shown.push(label);
        }
        shown.sort_unstable();
        shown.dedup();
        assert_eq!(shown.len(), MeasureSet::CATALOGUE.len(), "{shown:?}");
    }

    /// A name the catalogue does not carry is shown as it was written.
    #[test]
    fn a_measurement_the_catalogue_does_not_carry_is_shown_as_it_was_taken() {
        assert_eq!(label("largo_manga"), "largo_manga");
        assert_eq!(label(""), "");
    }
}
