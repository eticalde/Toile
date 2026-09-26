/// The anatomical station every vertex of a [`BodyMesh`] is tagged with,
/// which is what a place on the body is named by.
pub use toile_anny::Station;
use toile_anny::measure::Measures;
/// Anny's phenotype inputs: sex, age, build, muscle, height, proportions.
pub use toile_anny::phenotype::Phenotype;
/// Converts an age in years to the parameter [`Phenotype::age`] takes.
pub use toile_anny::phenotype::age_param_from_years;
/// Anny's own mesh, the one body producer Toile has: the same type
/// `toile_engine::draft::BodyMesh` re-exports.
pub use toile_anny::{BodyMesh, body_mesh};

use crate::draft::{BodyShape, MeasureSet};

/// Turning a body mesh into the signed distance field the solver collides
/// against.
pub mod bake;
/// The body's own measurement rings, and which one a garment belongs at.
mod belt;
pub use belt::Belt;
pub(crate) use belt::{at_station, worn_at};
/// Baked fields kept between runs, so a body is baked once and not once a run.
pub mod cache;
/// The body a drape falls on, and where a garment is let go over it.
mod collider;
pub use collider::{CLEARANCE, Collider};
/// The bake, on a thread of its own.
pub mod oven;
/// Solving the Anny body's levers against a measure set.
mod solve;
pub use solve::{AnnySolve, SolvedRow, solve_anny};
/// The line a tailor's tape lies along for each catalogue measurement.
mod tape;
pub use tape::{Tape, tape};

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

/// The ages a body can be given, in years: the asset is baked for adults only.
pub const ADULT_YEARS: std::ops::RangeInclusive<f64> = 18.0..=100.0;

/// The phenotype Anny generates a body from, out of the shape a document
/// stores.
///
/// A document does not range-check a shape, so every field is brought back
/// inside what the controls offer, and one that is not a number takes the
/// default body's value: a file edited by hand must not be able to ask the
/// evaluator for a body it was never baked for.
pub fn phenotype_of(shape: &BodyShape) -> Phenotype {
    let adult = BodyShape::default();
    let within = |value: f64, range: std::ops::RangeInclusive<f64>, fallback: f64| {
        if value.is_finite() {
            value.clamp(*range.start(), *range.end())
        } else {
            fallback
        }
    };
    let scale = |value: f64, fallback: f64| within(value, 0.0..=1.0, fallback);
    Phenotype {
        gender: scale(shape.sex, adult.sex),
        age: age_param_from_years(within(shape.age_years, ADULT_YEARS, adult.age_years)),
        muscle: scale(shape.muscle, adult.muscle),
        weight: scale(shape.build, adult.build),
        // The stature solve re-derives this from `estatura` before anything
        // reads it; only a body that carries no `estatura` keeps the midpoint.
        height: 0.5,
        proportions: scale(shape.proportions, adult.proportions),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A body that stores no shape is generated from the phenotype the tab
    /// always used, and a shape written out of range by hand is brought back
    /// inside what the asset was baked for.
    #[test]
    fn a_stored_shape_maps_onto_the_phenotype_inside_its_range() {
        assert_eq!(phenotype_of(&BodyShape::default()), Phenotype::default());
        let wild = BodyShape {
            sex: 7.0,
            age_years: f64::NAN,
            build: -1.0,
            muscle: f64::INFINITY,
            proportions: 0.25,
        };
        let expected = Phenotype {
            gender: 1.0,
            weight: 0.0,
            muscle: 0.5,
            proportions: 0.25,
            ..Phenotype::default()
        };
        assert_eq!(phenotype_of(&wild), expected);
        let aged = BodyShape {
            age_years: 140.0,
            ..BodyShape::default()
        };
        assert_eq!(
            phenotype_of(&aged).age.to_bits(),
            age_param_from_years(*ADULT_YEARS.end()).to_bits()
        );
    }
}
