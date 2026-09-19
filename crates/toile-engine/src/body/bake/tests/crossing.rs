use std::time::Instant;

use toile_anny::{BodyMesh, Station};

use super::{anny, brute, crossed_cubes};
use crate::body::bake::{Place, crossings};
use crate::body::{NO_LEVERS, Phenotype, body_mesh, default_measures, solve_anny};
use crate::golden::REFERENCE;

/// A triangle lying flat, one standing through the middle of it, and a third
/// that stands through it too but hangs off one of its corners. A centimetre
/// across, so the whole of it fits in the slab a sole is looked for in.
///
/// `tag` is the station every vertex carries.
fn pierced(tag: Station) -> BodyMesh {
    let centimetres: [[f32; 3]; 8] = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.25, -0.5, 0.25],
        [0.25, 0.5, 0.25],
        [0.9, 0.5, 0.9],
        [0.6, -0.5, 0.1],
        [0.6, 0.5, 0.1],
    ];
    let positions: Vec<f32> = centimetres.iter().flatten().map(|c| c * 0.01).collect();
    BodyMesh {
        normals: vec![0.0; positions.len()],
        positions,
        indices: vec![0, 1, 2, 3, 4, 5, 0, 6, 7],
        stations: vec![tag.tag(); centimetres.len()],
    }
}

#[test]
fn one_triangle_through_another_is_one_crossing_and_a_neighbour_is_none() {
    let mesh = pierced(Station::Hip);
    let found = crossings(&mesh);
    let [only] = found.all() else {
        panic!("one pair crosses, and {:?} were found", found.all());
    };
    assert_eq!(
        only.triangles,
        [0, 1],
        "the third goes through the first as well, off a corner they share"
    );
    assert_eq!(only.place, Place::Body([Station::Hip; 2]));
    assert_eq!(found.stations(), [Station::Hip]);
    // Halfway between a centroid at (1/3, 0, 1/3) and one at (1.4/3, 0.5/3,
    // 1.4/3), in centimetres.
    let middle = [0.004, 0.000_833, 0.004];
    for (got, want) in only.at.iter().zip(middle) {
        assert!((got - want).abs() < 1.0e-6, "{:?}", only.at);
    }
}

#[test]
fn a_crossing_is_a_sole_only_on_a_foot_and_only_against_the_floor() {
    let sole = crossings(&pierced(Station::Ankle));
    assert_eq!(sole.all()[0].place, Place::Sole);
    assert_eq!(sole.exposed().count(), 0, "nothing a garment can reach");
    assert!(sole.stations().is_empty());

    // The same foot with the floor a metre under it: some other part of the
    // mesh reaches lower, so this is an ankle and not a sole.
    let mut raised = pierced(Station::Ankle);
    raised.positions.extend([5.0, -1.0, 5.0]);
    raised.stations.push(Station::Ankle.tag());
    let found = crossings(&raised);
    assert_eq!(found.all()[0].place, Place::Body([Station::Ankle; 2]));
    assert_eq!(found.exposed().count(), 1);

    let mut untagged = pierced(Station::Ankle);
    untagged.stations.clear();
    assert_eq!(crossings(&untagged).all()[0].place, Place::Unplaced);
}

#[test]
fn a_mesh_with_nothing_to_look_at_reports_nothing() {
    let mut mesh = pierced(Station::Hip);
    mesh.positions[4] = f32::NAN;
    assert!(crossings(&mesh).all().is_empty());
    mesh.indices.clear();
    assert!(crossings(&mesh).all().is_empty());
}

#[test]
fn two_cubes_pushed_into_each_other_cross_where_brute_force_says_they_do() {
    let mesh = crossed_cubes();
    let found = crossings(&mesh);
    let tris = mesh.indices.as_chunks::<3>().0;
    assert_eq!(found.all().len(), brute::crossings(&mesh.positions, tris));
    assert!(found.all().len() >= 6, "{:?}", found.all());
    for c in found.all() {
        let [a, b] = c.triangles;
        assert!(a < 12 && b >= 12, "{a} and {b} are faces of one cube");
        assert_eq!(c.place, Place::Unplaced, "a cube carries no stations");
    }
    assert!(
        found.all().is_sorted_by_key(|c| c.triangles),
        "in rising order of triangle"
    );
    assert_eq!(found, crossings(&mesh), "and the same on every run");
}

/// The default body crosses itself 52 times, all of it under the soles, and
/// the body the goldens are taken against does not cross itself at all.
///
/// The count is held to the one a slower test with another method reaches
/// over the sole triangles alone, and this pass looks at the whole body: so
/// the 52 are also all there is.
#[test]
#[ignore = "release-only: brute force over the soles is millions of pairs"]
fn the_default_body_crosses_itself_only_under_its_soles_and_the_reference_nowhere() {
    let mesh = body_mesh(&Phenotype::default(), &NO_LEVERS);
    let started = Instant::now();
    let found = crossings(&mesh);
    println!(
        "crossings: {} pairs in {:.1} ms",
        found.all().len(),
        started.elapsed().as_secs_f64() * 1000.0
    );
    assert_eq!(
        found.all().len(),
        brute::crossings(&mesh.positions, &anny::soles(&mesh))
    );
    assert_eq!(found.all().len(), 52);
    assert_eq!(found.exposed().count(), 0, "every one of them is a sole");
    assert_eq!(found, crossings(&mesh), "the same pairs on every run");

    let reference = body_mesh(&REFERENCE, &NO_LEVERS);
    assert!(crossings(&reference).all().is_empty());
}

/// The body the app opens with is the default tape solved, not the default
/// phenotype bare, and it is the one that must never raise an alarm.
#[test]
fn the_body_the_app_opens_with_has_no_crossing_a_garment_can_reach() {
    let solved = solve_anny(&default_measures(), &Phenotype::default());
    let found = crossings(&body_mesh(&solved.phenotype, &solved.levers));
    assert!(
        !found.all().is_empty(),
        "its soles cross, as the bare one's do"
    );
    assert_eq!(found.exposed().count(), 0);
}

/// What the pass is for. A heavy, muscular male is inside the range the
/// controls offer, and his buttocks run into each other behind the crotch:
/// where a trouser's seat seam lies.
#[test]
fn a_heavy_muscular_male_crosses_himself_at_the_crotch() {
    let heavy = Phenotype {
        gender: 0.0,
        weight: 1.0,
        muscle: 1.0,
        ..Phenotype::default()
    };
    let found = crossings(&body_mesh(&heavy, &NO_LEVERS));
    assert_eq!(found.exposed().count(), 14);
    assert_eq!(found.stations(), [Station::Crotch]);
    for c in found.exposed() {
        assert!(c.at[0].abs() < 0.01, "on the midline: {:?}", c.at);
        assert!(c.at[2] < 0.0, "and behind it: {:?}", c.at);
    }
}
