use toile_body::{BodyMesh, BodyRes, PartialMeasures, body_mesh};

use crate::draft::MeasureSet;

/// The reference measure set the tab seeds with and the golden pins.
///
/// It uses the Spanish catalogue names (the user's data) with the
/// public-domain drafting-book defaults. Every catalogue name is written out
/// so the golden pins written values, never the derivation rules.
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
            ("cuello", 39.0),
            ("pecho", 96.0),
            ("pecho_alto", 94.0),
            ("bajo_pecho", 88.0),
            ("hombros", 46.0),
            ("brazo", 60.0),
            ("brazo_contorno", 30.0),
            ("muneca", 17.0),
            ("largo_espalda", 44.5),
            ("cabeza", 57.0),
        ],
    )
}

/// Reads the catalogue names from a measure set and lofts the body; any name
/// the set omits or leaves non-finite is derived from the stature and the
/// other values by proportion.
///
/// This is the one place the Spanish catalogue (data) meets the English
/// `BodyMeasures` (identifiers); the app never constructs `BodyMeasures`
/// itself.
pub fn body_from_measures(m: &MeasureSet) -> BodyMesh {
    let g = |name: &str| m.get(name).filter(|v| v.is_finite());
    let measures = PartialMeasures {
        height: g("estatura"),
        waist: g("cintura"),
        hip: g("cadera"),
        thigh: g("muslo"),
        knee: g("rodilla"),
        ankle: g("tobillo"),
        rise: g("tiro"),
        outseam: g("largo_lateral"),
        inseam: g("entrepierna"),
        hip_drop: g("altura_cadera"),
        neck: g("cuello"),
        bust: g("pecho"),
        upper_chest: g("pecho_alto"),
        underbust: g("bajo_pecho"),
        shoulder_width: g("hombros"),
        arm_length: g("brazo"),
        upper_arm: g("brazo_contorno"),
        wrist: g("muneca"),
        back_length: g("largo_espalda"),
        head: g("cabeza"),
    }
    .complete();
    body_mesh(&measures, BodyRes::default())
}
