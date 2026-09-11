use super::{geom, *};
use crate::body_mesh;
use crate::phenotype::Phenotype;

/// A ring edge on this mesh's scale is a few millimetres to a couple of
/// centimetres; two consecutive ring points farther apart than this cannot
/// be sitting on a shared crossed triangle, which means the loop is broken.
const MAX_RING_EDGE_M: f32 = 0.08;

/// No lever pulled — every measurement in this file is about the
/// phenotype-only body, not the solver.
const ZERO: [f64; 20] = [0.0; 20];

/// Checks every ring is either the documented single-point landmark or a
/// genuine closed loop over `positions`: non-empty, and every consecutive
/// pair of points (wrapping from the last back to the first) sits close
/// enough together to have come from one crossed triangle.
///
/// A ring's own vertex indices never change under any phenotype or lever
/// (see `measuring_different_phenotypes_does_not_change_the_ring_tables`),
/// but whether those indices still sit close enough to look like one
/// shared triangle can, in principle, break if a lever stretched its
/// vertices far apart — which is why this is reusable against the bare
/// template and against any morphed or solved mesh alike.
fn assert_rings_are_closed(positions: &[f32]) {
    for id in RingId::ALL {
        let points = ring_points(id);
        assert!(!points.is_empty(), "{id:?} is empty");
        if id.is_single_point() {
            assert_eq!(points.len(), 1, "{id:?} is documented as a single point");
            continue;
        }
        assert!(points.len() >= 3, "{id:?} has too few points to be a loop");
        let n = points.len();
        for i in 0..n {
            let a = geom::at(positions, points[i]);
            let b = geom::at(positions, points[(i + 1) % n]);
            let d = geom::dist(a, b);
            assert!(
                d < MAX_RING_EDGE_M,
                "{id:?}: points {i} and {} are {d:.4} m apart, not a shared triangle edge",
                (i + 1) % n
            );
        }
    }
}

#[test]
fn every_ring_is_a_single_point_or_a_closed_non_empty_loop() {
    assert_rings_are_closed(&decoded().positions);
}

/// The fourth slice's structural requirement: solving a lever all the way
/// to its own `±1` bound — the most a lever can ever stretch its vertices —
/// must not tear open the loop its own ring walks. Driven through
/// `crate::solve::solve_girth` rather than a hand-picked lever value, so
/// this exercises the real solver path, not just an arbitrary morph.
#[test]
fn rings_stay_closed_after_a_lever_is_solved_to_its_bound() {
    use crate::phenotype::LEVERS;

    let phenotype = Phenotype::default();
    let lever_id = |label: &str| {
        LEVERS
            .iter()
            .position(|&l| l == label)
            .expect("known lever label") as u8
    };
    let ids = [lever_id("waist-circ")];
    // Ask for something no waist can reach, so the secant runs to the +1
    // bound rather than settling short of it.
    let solved = crate::solve::solve_girth(&phenotype, &ZERO, &ids, 1000.0, |m| m.waist);
    assert!(solved.saturated, "1000 cm should saturate the waist lever");

    let mut levers = ZERO;
    levers[ids[0] as usize] = solved.value;
    let mesh = body_mesh(&phenotype, &levers);
    assert_rings_are_closed(&mesh.positions);
    let m = measure(&mesh.positions);
    assert!(
        m.hip_drop < m.rise,
        "altura_cadera {} is not shorter than tiro {} even at the waist's own bound",
        m.hip_drop,
        m.rise
    );
}

/// The baked `(vertex, vertex, t)` rings are read-only data: measuring two
/// wildly different bodies must never leave them changed, since a ring's
/// whole point is to be fixed geometry that only the *positions* it is
/// evaluated against move under.
#[test]
fn measuring_different_phenotypes_does_not_change_the_ring_tables() {
    let before = decoded().ring_points.clone();
    let _ = measure(&body_mesh(&Phenotype::default(), &ZERO).positions);
    let extreme = Phenotype {
        gender: 0.0,
        age: 1.0,
        muscle: 1.0,
        weight: 1.0,
        height: 1.0,
        proportions: 1.0,
    };
    let _ = measure(&body_mesh(&extreme, &ZERO).positions);
    assert_eq!(before, decoded().ring_points);
}

/// The waist-to-hip drop must always be shorter than the waist-to-crotch
/// rise. This is structural, not a tuning constant that happens to hold
/// for one reference body: [`RingId::Hip`] is baked strictly above
/// [`RingId::Crotch`] (the fork — see `crate::asset`'s doc), so however the
/// phenotype moves the waist, hip and crotch, the hip can never be as far
/// from the waist as the crotch is. Swept across gender, build and muscle
/// to demonstrate that, not just assert it for one body.
#[test]
fn hip_drop_is_always_shorter_than_rise() {
    for gender in [0.0, 0.5, 1.0] {
        for weight in [0.0, 0.5, 1.0] {
            for muscle in [0.0, 0.5, 1.0] {
                let phenotype = Phenotype {
                    gender,
                    age: 0.8,
                    muscle,
                    weight,
                    height: 0.5,
                    proportions: 0.5,
                };
                let m = measure(&body_mesh(&phenotype, &ZERO).positions);
                assert!(
                    m.hip_drop < m.rise,
                    "gender={gender} weight={weight} muscle={muscle}: \
                     hip_drop {} is not shorter than rise {}",
                    m.hip_drop,
                    m.rise
                );
            }
        }
    }
}

/// The property the next slice's one-dimensional, per-part solver depends
/// on: sweeping `weight` (build) from thinnest to heaviest, at every other
/// phenotype field fixed, must make the waist girth increase at every step.
/// If this ever stops holding, that solver cannot converge — so this test
/// fails loudly rather than the next slice discovering it the hard way.
#[test]
fn waist_girth_increases_monotonically_as_build_increases() {
    let mut previous: Option<f32> = None;
    for step in 0..=10 {
        let weight = f64::from(step) / 10.0;
        let phenotype = Phenotype {
            gender: 0.0,
            age: 0.8,
            muscle: 0.5,
            weight,
            height: 0.5,
            proportions: 0.5,
        };
        let waist = measure(&body_mesh(&phenotype, &ZERO).positions).waist;
        if let Some(before) = previous {
            assert!(
                waist > before,
                "waist did not increase at weight={weight}: {before} -> {waist}"
            );
        }
        previous = Some(waist);
    }
}

/// A generous but real range for every measurement on the reference adult
/// (male, age parameter 0.8, everything else Anny's neutral 0.5 — the same
/// phenotype `toile_engine::golden::anny_mesh_hash` pins). Bounds come from
/// the values this body actually measures, widened enough to catch a real
/// modelling mistake — an oblique cut inflating a limb by the ~30% the
/// module doc warns about, or a ring that caught the wrong loop — without
/// pinning the exact figure a golden already pins more precisely.
#[test]
fn the_reference_adult_measures_within_plausible_human_ranges() {
    let phenotype = Phenotype {
        gender: 0.0,
        age: 0.8,
        muscle: 0.5,
        weight: 0.5,
        height: 0.5,
        proportions: 0.5,
    };
    let m = measure(&body_mesh(&phenotype, &ZERO).positions);

    let in_range = |name: &str, v: f32, lo: f32, hi: f32| {
        assert!(
            (lo..=hi).contains(&v),
            "{name} = {v:.2} cm is outside the plausible range {lo}..={hi}"
        );
    };
    in_range("height", m.height, 175.0, 205.0);
    in_range("neck", m.neck, 32.0, 48.0);
    in_range("bust", m.bust, 90.0, 140.0);
    in_range("upper_chest", m.upper_chest, 85.0, 135.0);
    in_range("underbust", m.underbust, 80.0, 130.0);
    in_range("waist", m.waist, 70.0, 110.0);
    in_range("hip", m.hip, 85.0, 120.0);
    in_range("thigh", m.thigh, 45.0, 70.0);
    in_range("knee", m.knee, 30.0, 50.0);
    in_range("ankle", m.ankle, 18.0, 35.0);
    in_range("upper_arm", m.upper_arm, 22.0, 50.0);
    in_range("wrist", m.wrist, 15.0, 21.0);
    in_range("head", m.head, 52.0, 62.0);
    in_range("rise", m.rise, 25.0, 36.0);
    in_range("hip_drop", m.hip_drop, 16.0, 24.0);
    in_range("inseam", m.inseam, 65.0, 95.0);
    in_range("outseam", m.outseam, 90.0, 140.0);
    // Short of a generic anthropometric table even at the best landmark an
    // honest search finds — see `measure`'s own doc for the evidence this
    // is this mesh's own proportion, not a ring placement bug.
    in_range("back_length", m.back_length, 28.0, 40.0);
    in_range("arm_length", m.arm_length, 58.0, 68.0);
    in_range("shoulder_width", m.shoulder_width, 35.0, 55.0);
}

/// `RingId::Bust` is cut where the breast targets themselves say the apex
/// sits (`crate::asset`'s doc), so a female-default body's breast should
/// show up as a real difference between `pecho` and `bajo_pecho` — not the
/// under-3-cm gap the old ribcage-only scan produced. Male is checked too,
/// as a guard against the fix instead flattening the male reading.
#[test]
fn the_female_default_shows_a_real_bust_apex() {
    let female = Phenotype {
        gender: 1.0,
        ..Phenotype::default()
    };
    let m = measure(&body_mesh(&female, &ZERO).positions);
    let gap = m.bust - m.underbust;
    assert!(
        (9.0..=16.0).contains(&gap),
        "female bust - underbust = {gap:.2} cm, expected roughly 10-15"
    );

    let male = Phenotype {
        gender: 0.0,
        age: 0.8,
        muscle: 0.5,
        weight: 0.5,
        height: 0.5,
        proportions: 0.5,
    };
    let m = measure(&body_mesh(&male, &ZERO).positions);
    let gap = m.bust - m.underbust;
    assert!(
        (3.0..=12.0).contains(&gap),
        "male bust - underbust = {gap:.2} cm moved out of a plausible range"
    );
}
