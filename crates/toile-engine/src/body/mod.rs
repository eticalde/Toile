use toile_anny::measure::Measures;
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

/// One catalogue name and the field of a fresh [`Measures`] that answers it.
///
/// The reading is a bare `fn` rather than an option, so a row naming nothing
/// on the body cannot be written down at all.
pub(crate) struct Reading {
    /// The catalogue's own Spanish name, as the document writes it.
    pub name: &'static str,
    /// Where that name is read off a measured body.
    pub read: fn(&Measures) -> f32,
}

impl Reading {
    /// One row, on one line, so the table below can be read as a table.
    const fn of(name: &'static str, read: fn(&Measures) -> f32) -> Reading {
        Reading { name, read }
    }
}

/// The one mapping from a catalogue name to what the body measures there.
///
/// The *medido* column and the solver's own target function both read through
/// this table and no other, so the two cannot come to read different fields
/// for the same name. They were a pair of hand-written matches over the same
/// struct, kept in step by a comment saying they had to be.
pub(crate) const READINGS: [Reading; 20] = [
    Reading::of("estatura", |m| m.height),
    Reading::of("cuello", |m| m.neck),
    Reading::of("pecho", |m| m.bust),
    Reading::of("pecho_alto", |m| m.upper_chest),
    Reading::of("bajo_pecho", |m| m.underbust),
    Reading::of("cintura", |m| m.waist),
    Reading::of("cadera", |m| m.hip),
    Reading::of("muslo", |m| m.thigh),
    Reading::of("rodilla", |m| m.knee),
    Reading::of("tobillo", |m| m.ankle),
    Reading::of("brazo_contorno", |m| m.upper_arm),
    Reading::of("muneca", |m| m.wrist),
    Reading::of("cabeza", |m| m.head),
    Reading::of("tiro", |m| m.rise),
    Reading::of("altura_cadera", |m| m.hip_drop),
    Reading::of("entrepierna", |m| m.inseam),
    Reading::of("largo_lateral", |m| m.outseam),
    Reading::of("largo_espalda", |m| m.back_length),
    Reading::of("brazo", |m| m.arm_length),
    Reading::of("hombros", |m| m.shoulder_width),
];

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
        READINGS.map(|row| (row.name, f64::from((row.read)(&m)))),
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
///
/// The very number [`measured_anny`] reports as `estatura`, because it is the
/// same call: the panel prints the two beside each other, and a second scan
/// written out here could be changed on its own.
pub fn stature_cm(mesh: &BodyMesh) -> f32 {
    toile_anny::measure::height_cm(&mesh.positions)
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
