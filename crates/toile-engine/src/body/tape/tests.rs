use toile_anny::measure::{at, floor_and_crown, measure, path, ring};

use super::*;
use crate::body::{READINGS, body_mesh, default_measures, phenotype_of, solve_anny};
use crate::draft::{BodyShape, MeasureSet};

mod on_skin;

/// How far a tape's length may stand from its measurement, in centimetres:
/// one micrometre.
///
/// The two are the same chords over the same f32 points, summed once in f32
/// and once in f64, so what is left between them is rounding. It is the same
/// bits on every architecture, since nothing on either path fuses a
/// multiply-add, so the margin cannot shrink on another machine. Every real
/// mistake is far above it: a girth loop left unflattened reads micrometres
/// long at the wrist and centimetres at the upper chest, a path missing its
/// last chord or the inseam missing its drop to the floor is millimetres
/// short at the least.
const TOLERANCE_CM: f64 = 0.0001;

/// The body the Anny goldens pin: male, age parameter 0.8, the rest neutral.
fn reference() -> BodyMesh {
    let phenotype = toile_anny::phenotype::Phenotype {
        gender: 0.0,
        age: 0.8,
        muscle: 0.5,
        weight: 0.5,
        height: 0.5,
        proportions: 0.5,
    };
    body_mesh(&phenotype, &[0.0; 20])
}

/// The body Toile's own default tape solves to, levers pulled.
fn solved() -> BodyMesh {
    let solve = solve_anny(&default_measures(), &phenotype_of(&BodyShape::default()));
    body_mesh(&solve.phenotype, &solve.levers)
}

/// What is drawn is what is measured: on a phenotype-only body and on a
/// solved one, every catalogue name has a tape, and that tape is as long as
/// the number the panel reports for it.
#[test]
fn every_catalogue_tape_is_as_long_as_its_measurement() {
    for (body, mesh) in [("reference", reference()), ("solved", solved())] {
        let measured = measure(&mesh.positions);
        for name in MeasureSet::CATALOGUE {
            let drawn = tape(name, &mesh).unwrap_or_else(|| panic!("{name} has no tape"));
            let reading = READINGS
                .iter()
                .find(|row| row.name == name)
                .unwrap_or_else(|| panic!("{name} has no reading"));
            let want = f64::from((reading.read)(&measured));
            let got = drawn.length_cm();
            assert!(
                (got - want).abs() <= TOLERANCE_CM,
                "{body} {name}: the tape is {got:.5} cm, the measurement {want:.5} cm"
            );
        }
    }
}

#[test]
fn a_name_outside_the_catalogue_has_no_tape() {
    assert_eq!(tape("no_es_una_medida", &reference()), None);
}

/// A length's tape, before it is lifted, is not near the skin but on it: the
/// very points `measure` walks, bit for bit, and for the inseam one more,
/// plumb under the last at the floor's height.
#[test]
#[allow(
    clippy::float_cmp,
    reason = "the tape is built from these exact evaluations, not approximated"
)]
fn a_length_is_drawn_through_the_points_it_is_measured_along() {
    let mesh = solved();
    let (floor, _) = floor_and_crown(&mesh.positions);
    for name in MeasureSet::CATALOGUE {
        let Some(Lay::Skin(id)) = lay(name) else {
            continue;
        };
        let drawn = tape(name, &mesh).expect("a catalogue name");
        let skin: Vec<[f32; 3]> = path(id).iter().map(|&p| at(&mesh.positions, p)).collect();
        assert_eq!(&drawn.points[..skin.len()], skin.as_slice(), "{name}");
        if id.ends_on_floor() {
            let [x, _, z] = skin[skin.len() - 1];
            assert_eq!(drawn.points[skin.len()..], [[x, floor, z]], "{name}");
        } else {
            assert_eq!(drawn.points.len(), skin.len(), "{name}");
        }
    }
}

/// On the body the default tape solves to, the leg and arm tapes still run
/// down the right side, where the asset baked them, and never over to the
/// left leg or arm.
#[test]
fn the_leg_and_arm_tapes_stay_on_the_right_of_the_solved_body() {
    let mesh = solved();
    for name in [
        "entrepierna",
        "largo_lateral",
        "tiro",
        "altura_cadera",
        "brazo",
    ] {
        let drawn = tape(name, &mesh).expect("a catalogue name");
        for p in &drawn.points {
            assert!(p[0] < 0.0, "{name} reaches {p:?}, left of the midline");
        }
    }
}

/// The stature rule stands beside the body, not through it.
#[test]
fn the_stature_tape_stands_beside_the_body() {
    let mesh = solved();
    let drawn = tape("estatura", &mesh).expect("a catalogue name");
    for &x in mesh.positions.iter().step_by(3) {
        assert!(drawn.points.iter().all(|p| p[0] <= x));
    }
}

/// A girth is drawn through the very crossings its ring is cut at on this
/// body, bit for bit, and measured through those same crossings flattened:
/// both lines have a point for every crossing, in the ring's order.
#[test]
#[allow(
    clippy::float_cmp,
    reason = "the drawn line is built from these exact evaluations, not approximated"
)]
fn a_girth_is_drawn_through_its_ring_crossings_on_the_skin() {
    let mesh = solved();
    for name in MeasureSet::CATALOGUE {
        let Some(Lay::Girth(id)) = lay(name) else {
            continue;
        };
        let drawn = tape(name, &mesh).expect("a catalogue name");
        let crossings = ring(id).points;
        let skin: Vec<[f32; 3]> = crossings.iter().map(|&p| at(&mesh.positions, p)).collect();
        assert_eq!(drawn.drawn, skin, "{name}");
        assert_eq!(drawn.points.len(), skin.len(), "{name}");
    }
}

/// A lift moves every drawn point the gap asked for along the skin's unit
/// normal there and nowhere else, so the tape stands as far off every body.
#[test]
fn a_lift_moves_every_drawn_point_the_gap_off_the_skin() {
    let mesh = solved();
    for name in ["cintura", "largo_lateral"] {
        let drawn = tape(name, &mesh).expect("a catalogue name");
        let lifted = drawn.lifted(0.003);
        for ((l, p), o) in lifted.iter().zip(&drawn.drawn).zip(&drawn.out) {
            let moved = [0, 1, 2].map(|k| l[k] - p[k] - 0.003 * o[k]);
            let unit = o[0] * o[0] + o[1] * o[1] + o[2] * o[2];
            assert!(moved.iter().all(|m| m.abs() < 1.0e-6), "{name}: {moved:?}");
            assert!((unit - 1.0).abs() < 1.0e-5, "{name}: |out|² = {unit}");
        }
    }
}
