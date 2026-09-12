/// Anny's phenotype inputs: sex, age, build, muscle, height, proportions.
pub use toile_anny::phenotype::Phenotype;
/// Converts an age in years to the parameter [`Phenotype::age`] takes.
pub use toile_anny::phenotype::age_param_from_years;
/// Anny's own mesh, the one body producer Toile has: the same type
/// `toile_engine::draft::BodyMesh` re-exports.
pub use toile_anny::{BodyMesh, Station, body_mesh};

use crate::draft::MeasureSet;

/// Solving the Anny body's levers against a measure set.
mod solve;
pub use solve::{AnnySolve, SolvedRow, solve_anny};

/// The lever vector every row is at, with no name pulled — the state a
/// freshly chosen phenotype starts from before anything is solved.
pub const NO_LEVERS: [f64; 20] = [0.0; 20];

/// Reads every catalogue measurement directly off a generated Anny mesh's
/// own positions, keyed by the catalogue's Spanish names.
///
/// This is the *medido* half of the tab's dado/medido/Δ row, alongside
/// [`default_measures`]'s dado.
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

/// The reference measure set the tab seeds a fresh mannequin with and the
/// golden pins.
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

/// The mesh's bounding-box height along y (up), in centimetres.
///
/// Anny's own `height` phenotype input is not centimetres by itself (it is
/// the `[0, 1]` position between `minheight` and `maxheight`), so this is
/// the honest number to show next to the body: the stature the generated
/// mesh actually measures.
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

    /// The tags the baker wrote are the ones the mapping reads: the
    /// reference body carries a vertex for every station a measurement
    /// names.
    #[test]
    fn the_body_carries_every_station_the_mapping_names() {
        let mesh = body_mesh(&Phenotype::default(), &NO_LEVERS);
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
