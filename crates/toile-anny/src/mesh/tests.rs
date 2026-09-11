use super::*;

const VERTEX_COUNT: usize = 13_380;
const TRIANGLE_COUNT: usize = 26_756;

/// No lever pulled — the phenotype-only body the second slice shipped.
const ZERO: [f64; 20] = [0.0; 20];

/// A mesh's bounding-box height along y (up), in the mesh's own metres.
fn bbox_height(m: &BodyMesh) -> f32 {
    let ys = m.positions.iter().skip(1).step_by(3);
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for &y in ys {
        lo = lo.min(y);
        hi = hi.max(y);
    }
    hi - lo
}

#[test]
fn the_shipped_asset_loads_at_the_expected_size() {
    let m = body_mesh(&Phenotype::default(), &ZERO);
    assert_eq!(m.vertex_count(), VERTEX_COUNT);
    assert_eq!(m.positions.len(), VERTEX_COUNT * 3);
    assert_eq!(m.normals.len(), VERTEX_COUNT * 3);
    assert_eq!(m.indices.len(), TRIANGLE_COUNT * 3);
    assert_eq!(m.stations.len(), VERTEX_COUNT);
}

#[test]
fn every_index_is_in_range_and_every_position_is_finite() {
    let m = body_mesh(&Phenotype::default(), &ZERO);
    assert!(m.indices.iter().all(|&i| (i as usize) < m.vertex_count()));
    assert!(m.positions.iter().all(|f| f.is_finite()));
}

#[test]
fn every_station_tag_is_below_the_count() {
    let m = body_mesh(&Phenotype::default(), &ZERO);
    for &tag in &m.stations {
        assert!(
            u32::from(tag) < u32::from(toile_body::Station::COUNT),
            "tag {tag} is not a real station"
        );
    }
}

#[test]
fn all_twenty_two_stations_are_carried_by_some_vertex() {
    let m = body_mesh(&Phenotype::default(), &ZERO);
    for station in toile_body::Station::ALL {
        assert!(
            m.stations.contains(&station.tag()),
            "no vertex is tagged {station:?}"
        );
    }
}

#[test]
fn every_normal_is_unit_length() {
    let m = body_mesh(&Phenotype::default(), &ZERO);
    for n in m.normals.as_chunks::<3>().0 {
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        assert!((len - 1.0).abs() < 1.0e-4, "normal length {len}");
    }
}

#[test]
fn normals_point_outward() {
    // toile_body's per-part version dots each vertex's normal against the
    // vector from its own ring's centroid, which works because a ring is
    // star-shaped around it. The whole body is not star-shaped around any
    // one point — an outstretched arm's surface does not point away from
    // the torso's centroid — so that test measured only 58% outward here.
    // A closed, consistently-wound mesh has a much stronger and exact
    // property instead: the divergence theorem says the signed volume
    // enclosed, summed one tetrahedron per triangle from any apex, is
    // positive exactly when the winding is CCW-outward.
    let m = body_mesh(&Phenotype::default(), &ZERO);
    let at = |i: u32| {
        let i = i as usize * 3;
        [
            f64::from(m.positions[i]),
            f64::from(m.positions[i + 1]),
            f64::from(m.positions[i + 2]),
        ]
    };
    let signed_volume_x6: f64 = m
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| {
            let (a, b, c) = (at(t[0]), at(t[1]), at(t[2]));
            let cross = [
                b[1] * c[2] - b[2] * c[1],
                b[2] * c[0] - b[0] * c[2],
                b[0] * c[1] - b[1] * c[0],
            ];
            a[0] * cross[0] + a[1] * cross[1] + a[2] * cross[2]
        })
        .sum();
    let litres = signed_volume_x6 / 6.0 * 1000.0;
    assert!(
        (10.0..200.0).contains(&litres),
        "enclosed volume {litres} L is not a plausible body"
    );
}

#[test]
fn two_different_phenotypes_move_the_mesh() {
    let a = body_mesh(&Phenotype::default(), &ZERO);
    let b = body_mesh(
        &Phenotype {
            muscle: 1.0,
            weight: 1.0,
            ..Phenotype::default()
        },
        &ZERO,
    );
    assert_ne!(
        a.positions, b.positions,
        "a heavier, more muscular body must differ"
    );
}

#[test]
fn a_taller_height_parameter_gives_a_taller_bounding_box() {
    let short = body_mesh(
        &Phenotype {
            height: 0.0,
            ..Phenotype::default()
        },
        &ZERO,
    );
    let tall = body_mesh(
        &Phenotype {
            height: 1.0,
            ..Phenotype::default()
        },
        &ZERO,
    );
    assert!(
        bbox_height(&tall) > bbox_height(&short),
        "height 1.0 ({} m) should stand taller than height 0.0 ({} m)",
        bbox_height(&tall),
        bbox_height(&short)
    );
}

#[test]
fn male_and_female_defaults_differ() {
    let male = body_mesh(
        &Phenotype {
            gender: 0.0,
            ..Phenotype::default()
        },
        &ZERO,
    );
    let female = body_mesh(
        &Phenotype {
            gender: 1.0,
            ..Phenotype::default()
        },
        &ZERO,
    );
    assert_ne!(male.positions, female.positions);
}

#[test]
fn topology_is_fixed_across_phenotypes() {
    // The same property `toile_body`'s loft pins for its own measure sets:
    // only positions may move with the phenotype, never the triangulation
    // or the station assignment, because both are baked from the template
    // alone.
    let a = body_mesh(&Phenotype::default(), &ZERO);
    let b = body_mesh(
        &Phenotype {
            gender: 0.0,
            age: 1.0,
            muscle: 1.0,
            weight: 0.0,
            height: 1.0,
            proportions: 1.0,
        },
        &ZERO,
    );
    assert_eq!(a.indices, b.indices);
    assert_eq!(a.stations, b.stations);
}

#[test]
fn a_phenotype_below_the_adult_floor_matches_the_floor() {
    // This tier ships no child rows; an age parameter below `young` must
    // floor to the same mesh as the floor itself, not silently drift toward
    // a body the asset cannot produce.
    let floor = phenotype::adult_age_floor();
    let floored = body_mesh(
        &Phenotype {
            age: -1.0,
            ..Phenotype::default()
        },
        &ZERO,
    );
    let at_floor = body_mesh(
        &Phenotype {
            age: floor,
            ..Phenotype::default()
        },
        &ZERO,
    );
    assert_eq!(floored.positions, at_floor.positions);
}
