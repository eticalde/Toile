use toile_body::{BodyMeasures, BodyMesh, BodyRes, body_mesh};

use crate::draft::MeasureSet;

/// The reference measure set the tab seeds with and the golden pins.
///
/// It uses the Spanish catalogue names (the user's data) with the
/// public-domain drafting-book defaults.
pub fn default_measures() -> MeasureSet {
    MeasureSet::new(
        "Etienne",
        [
            ("cintura", 84.0),
            ("cadera", 98.0),
            ("muslo", 58.0),
            ("rodilla", 40.0),
            ("tobillo", 24.0),
            ("tiro", 27.0),
            ("largo_lateral", 104.0),
            ("entrepierna", 78.0),
            ("altura_cadera", 20.0),
            ("estatura", 178.0),
        ],
    )
}

/// Reads the catalogue names from a measure set and lofts the body, falling
/// back to the default for any name the set omits or leaves non-finite.
///
/// This is the one place the Spanish catalogue (data) meets the English
/// `BodyMeasures` (identifiers); the app never constructs `BodyMeasures`
/// itself.
pub fn body_from_measures(m: &MeasureSet) -> BodyMesh {
    let d = BodyMeasures::default();
    let g = |name: &str, fallback: f64| m.get(name).filter(|v| v.is_finite()).unwrap_or(fallback);
    let measures = BodyMeasures {
        height: g("estatura", d.height),
        waist: g("cintura", d.waist),
        hip: g("cadera", d.hip),
        thigh: g("muslo", d.thigh),
        knee: g("rodilla", d.knee),
        ankle: g("tobillo", d.ankle),
        rise: g("tiro", d.rise),
        outseam: g("largo_lateral", d.outseam),
        inseam: g("entrepierna", d.inseam),
        hip_drop: g("altura_cadera", d.hip_drop),
    };
    body_mesh(&measures, BodyRes::default())
}
