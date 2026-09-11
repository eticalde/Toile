/// Anny's phenotype inputs (sex, age, build, muscle, height, proportions),
/// re-exported here rather than left for callers to reach through
/// `toile-anny` directly — the app depends on `toile-engine` only, and this
/// is the one type its Anny controls need.
pub use toile_anny::phenotype::Phenotype;
/// Converts an age in years to the parameter [`Phenotype::age`] takes.
pub use toile_anny::phenotype::age_param_from_years;
use toile_body::{BodyMesh, BodyRes, PartialMeasures, Station, body_mesh};

use crate::draft::MeasureSet;

/// Writing a value into a measures row and solving the Anny body's levers
/// until it agrees — one lever (or tied pair) per row, a fixed order, and
/// an honest "tope del modelo" when the target is out of the model's reach.
mod solve;
pub use solve::{AnnySolve, SolvedRow, solve_anny};

/// The lever vector every row is at, with no name pulled — the state a
/// freshly chosen phenotype starts from before anything is solved.
pub const NO_LEVERS: [f64; 20] = [0.0; 20];

/// Reads every catalogue measurement directly off a generated Anny mesh's
/// own positions, keyed by the catalogue's Spanish names.
///
/// This is the *medido* half of the tab's dado/medido/Δ row, alongside
/// [`default_measures`]'s dado. `mesh` must be the Anny model's own output
/// (see `body_from_measures_with`): the tailor's dummy has no such ring
/// data and [`toile_anny::measure::measure`] would panic on its
/// differently-shaped positions.
pub fn measured_anny(mesh: &BodyMesh) -> MeasureSet {
    let m = toile_anny::measure::measure(&mesh.positions);
    MeasureSet::new(
        "Medido",
        [
            ("estatura", f64::from(m.height)),
            ("cuello", f64::from(m.neck)),
            ("pecho", f64::from(m.bust)),
            ("pecho_alto", f64::from(m.upper_chest)),
            ("bajo_pecho", f64::from(m.underbust)),
            ("cintura", f64::from(m.waist)),
            ("cadera", f64::from(m.hip)),
            ("muslo", f64::from(m.thigh)),
            ("rodilla", f64::from(m.knee)),
            ("tobillo", f64::from(m.ankle)),
            ("brazo_contorno", f64::from(m.upper_arm)),
            ("muneca", f64::from(m.wrist)),
            ("cabeza", f64::from(m.head)),
            ("tiro", f64::from(m.rise)),
            ("altura_cadera", f64::from(m.hip_drop)),
            ("entrepierna", f64::from(m.inseam)),
            ("largo_lateral", f64::from(m.outseam)),
            ("largo_espalda", f64::from(m.back_length)),
            ("brazo", f64::from(m.arm_length)),
            ("hombros", f64::from(m.shoulder_width)),
        ],
    )
}

/// Which mesh producer the Maniquies tab loads.
///
/// Both return the same [`BodyMesh`] shape, so the viewport, the camera and
/// the station highlight work identically regardless of which one is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BodyModel {
    /// The procedural loft: exact to the tape, no asset, microseconds.
    #[default]
    TailorDummy,
    /// The Anny body baked from CC0 MakeHuman/MPFB2 data, morphed by a
    /// [`Phenotype`] and solved toward the tape's own catalogue values —
    /// see [`solve_anny`] and `body_from_measures_with`'s doc.
    Anny,
}

/// The stations a catalogue measurement is read at: the region the interface
/// lights while the person handles that measurement.
///
/// A girth is its own station; a length is every station its tape runs past;
/// the stature is the whole body. A name outside the catalogue lights nothing.
pub fn stations_for(name: &str) -> &'static [Station] {
    use Station::{
        Ankle, Armpit, Biceps, Bust, Calf, Cheek, Crotch, Crown, Deltoid, Elbow, Forearm, HeadMax,
        Hip, Knee, NeckBase, NeckTop, Shoulder, Thigh, Underbust, Waist, Wrist,
    };
    match name {
        "cintura" => &[Waist],
        "cadera" => &[Hip],
        "muslo" => &[Thigh],
        "rodilla" => &[Knee],
        "tobillo" => &[Ankle],
        "tiro" => &[Crotch, Hip, Waist],
        "largo_lateral" => &[Ankle, Calf, Knee, Thigh, Crotch, Hip, Waist],
        "entrepierna" => &[Ankle, Calf, Knee, Thigh],
        "altura_cadera" => &[Hip, Waist],
        "estatura" => &Station::ALL,
        "cuello" => &[NeckBase, NeckTop],
        "pecho" => &[Bust],
        "pecho_alto" => &[Armpit],
        "bajo_pecho" => &[Underbust],
        "hombros" => &[Shoulder, Deltoid],
        "brazo" => &[Wrist, Forearm, Elbow, Biceps, Deltoid],
        "brazo_contorno" => &[Biceps],
        "muneca" => &[Wrist],
        "largo_espalda" => &[Waist, Underbust, Bust, Armpit, Shoulder, NeckBase],
        "cabeza" => &[Cheek, HeadMax, Crown],
        _ => &[],
    }
}

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

/// Lofts a body with the chosen model.
///
/// `TailorDummy` reads `m` exactly as [`body_from_measures`] does and
/// ignores `anny`/`levers` — the tailor's dummy has no levers at all, and
/// hits every girth by construction. `Anny` ignores `m` directly — a
/// [`MeasureSet`] cannot express sex or age, so the Maniquies tab owns a
/// `Phenotype` value of its own and hands it here, the same way it owns
/// `m` — and morphs the baked template by `anny` and `levers` (see
/// [`solve_anny`] for how `levers` gets its values from `m`). The two
/// producers never share a dependency: `toile-anny`'s own `BodyMesh` is
/// field-for-field identical to `toile_body`'s, so this is the one place
/// their values are moved across.
pub fn body_from_measures_with(
    model: BodyModel,
    m: &MeasureSet,
    anny: &Phenotype,
    levers: &[f64; 20],
) -> BodyMesh {
    match model {
        BodyModel::TailorDummy => body_from_measures(m),
        BodyModel::Anny => {
            let mesh = toile_anny::body_mesh(anny, levers);
            BodyMesh {
                positions: mesh.positions,
                normals: mesh.normals,
                indices: mesh.indices,
                stations: mesh.stations,
            }
        }
    }
}

/// The mesh's bounding-box height along y (up), in centimetres.
///
/// Anny's own `height` phenotype input is not centimetres by itself (it is
/// the `[0, 1]` position between `minheight` and `maxheight`), so this is
/// the honest number to show next to that slider: the stature the
/// generated body actually measures.
pub fn stature_cm(mesh: &BodyMesh) -> f32 {
    let ys = mesh.positions.iter().skip(1).step_by(3);
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for &y in ys {
        lo = lo.min(y);
        hi = hi.max(y);
    }
    (hi - lo) * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every catalogue name lights somewhere on the body, and the stature
    /// lights all of it; a stray name lights nothing rather than something.
    #[test]
    fn every_catalogue_measurement_lights_a_region() {
        for name in MeasureSet::CATALOGUE {
            assert!(!stations_for(name).is_empty(), "{name} lights nothing");
        }
        assert_eq!(stations_for("estatura").len(), usize::from(Station::COUNT));
        assert!(stations_for("no_es_una_medida").is_empty());
    }

    /// The tags the loft writes are the ones the mapping reads: the reference
    /// body carries a vertex for every station a measurement names.
    #[test]
    fn the_body_carries_every_station_the_mapping_names() {
        let mesh = body_from_measures(&default_measures());
        for name in MeasureSet::CATALOGUE {
            for station in stations_for(name) {
                assert!(
                    mesh.stations.contains(&station.tag()),
                    "{name}: no vertex is tagged {station:?}"
                );
            }
        }
    }
}
