use super::{BakeError, Lattice, inspect, mesh_sdf};

mod anny;
mod brute;
mod cavity;

/// The cube's side, in metres.
const SIDE: f32 = 0.2;

/// Coarse enough that a debug build bakes it in a blink, fine enough that
/// the answers below are several voxels apart.
const CELL: f64 = 0.01;

/// The same band the body is baked with.
const BAND: f64 = 0.025;

/// The unit cube's corners, in the order [`FACES`] indexes them.
const CORNERS: [[f32; 3]; 8] = [
    [0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [1.0, 1.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
    [1.0, 0.0, 1.0],
    [1.0, 1.0, 1.0],
    [0.0, 1.0, 1.0],
];

/// Two triangles a face, every one wound counter-clockwise seen from
/// outside: bottom, top, front, right, back, left.
const FACES: [[u32; 3]; 12] = [
    [0, 3, 2],
    [0, 2, 1],
    [4, 5, 6],
    [4, 6, 7],
    [0, 1, 5],
    [0, 5, 4],
    [1, 2, 6],
    [1, 6, 5],
    [2, 3, 7],
    [2, 7, 6],
    [3, 0, 4],
    [3, 4, 7],
];

/// One float's bits, for the answers that are exact rather than close.
///
/// Past the band every sample is `±band` and nothing else, and the grid
/// carries back the very cell it was given, so those two are equalities. A
/// tolerance there would pass on a field that had drifted.
fn bits(f: f32) -> u32 {
    f.to_bits()
}

/// A closed cube of side [`SIDE`], its near corner at the origin.
fn cube() -> (Vec<f32>, Vec<u32>) {
    (
        CORNERS.iter().flatten().map(|c| c * SIDE).collect(),
        FACES.iter().flatten().copied().collect(),
    )
}

#[test]
fn a_mesh_with_nothing_in_it_or_a_position_that_is_not_a_number_is_refused() {
    assert_eq!(
        mesh_sdf(&[], &[], CELL, BAND).unwrap_err(),
        BakeError::Empty
    );
    let (mut positions, indices) = cube();
    positions[0] = f32::NAN;
    assert_eq!(
        mesh_sdf(&positions, &indices, CELL, BAND).unwrap_err(),
        BakeError::NotFinite
    );
}

#[test]
fn a_mesh_in_the_wrong_unit_is_refused_rather_than_allocated() {
    // The very same cube, its positions read as millimetres: closed,
    // orientable, and two hundred metres across, which is twenty thousand
    // samples an axis and eight million million in all.
    let (positions, indices) = cube();
    let millimetres: Vec<f32> = positions.iter().map(|c| c * 1000.0).collect();
    assert!(matches!(
        mesh_sdf(&millimetres, &indices, CELL, BAND),
        Err(BakeError::TooLarge(_))
    ));
    assert!(
        mesh_sdf(&positions, &indices, CELL, BAND).is_ok(),
        "and the same cube in metres still bakes"
    );
}

#[test]
fn a_cube_with_a_face_torn_out_is_refused() {
    let (positions, mut indices) = cube();
    indices.truncate(indices.len() - 6);
    let torn = inspect(&positions, &indices);
    assert_eq!(torn.boundary_edges, 4, "the rim of the hole");
    assert_eq!(torn.euler, 1);
    assert!(!torn.earns_the_sign());
    assert!(matches!(
        mesh_sdf(&positions, &indices, CELL, BAND),
        Err(BakeError::NotClosed(_))
    ));
}

#[test]
fn a_single_flipped_face_is_refused() {
    let (positions, mut indices) = cube();
    indices.swap(0, 1);
    let flipped = inspect(&positions, &indices);
    assert_eq!(flipped.inconsistent_edges, 3, "the flipped triangle's own");
    assert_eq!(flipped.boundary_edges, 0, "a flip tears nothing");
    assert!(matches!(
        mesh_sdf(&positions, &indices, CELL, BAND),
        Err(BakeError::NotClosed(_))
    ));
}

#[test]
fn a_cube_wound_inside_out_is_refused() {
    // Consistent with itself and closed, so only the enclosed volume can
    // tell: the sign method reads the faces as pointing outward, and here
    // they all point in.
    let (positions, indices) = cube();
    let inverted: Vec<u32> = indices
        .as_chunks::<3>()
        .0
        .iter()
        .flat_map(|t| [t[0], t[2], t[1]])
        .collect();
    let m = inspect(&positions, &inverted);
    assert_eq!(m.inconsistent_edges, 0);
    assert!(m.litres < 0.0, "{} litres", m.litres);
    assert!(!m.earns_the_sign());
}

#[test]
fn a_closed_cube_earns_the_sign() {
    let (positions, indices) = cube();
    let m = inspect(&positions, &indices);
    assert_eq!((m.vertices, m.triangles, m.edges), (8, 12, 18));
    assert_eq!(m.boundary_edges, 0);
    assert_eq!(m.nonmanifold_edges, 0);
    assert_eq!(m.inconsistent_edges, 0);
    assert_eq!(m.degenerate_triangles, 0);
    assert_eq!(m.euler, 2, "one closed surface of genus 0");
    assert!((m.litres - 8.0).abs() < 1.0e-6, "{} litres", m.litres);
    assert!(m.earns_the_sign());
}

#[test]
fn the_field_reads_the_distance_to_the_skin_and_saturates_past_the_band() {
    let (positions, indices) = cube();
    let field = mesh_sdf(&positions, &indices, CELL, BAND).expect("the cube is closed");
    let half = SIDE / 2.0;
    assert_eq!(
        bits(field.sample(half, half, half)),
        bits(-(BAND as f32)),
        "the core is further in than the band reaches, and reads {}",
        field.sample(half, half, half)
    );
    assert_eq!(
        bits(field.sample(-1.0, -1.0, -1.0)),
        bits(BAND as f32),
        "a metre away reads {}",
        field.sample(-1.0, -1.0, -1.0)
    );
    let at_depth = |z: f32| f64::from(field.sample(0.1, 0.1, z));
    assert!(
        (at_depth(0.005) + 0.005).abs() < 1.0e-6,
        "{}",
        at_depth(0.005)
    );
    assert!(
        (at_depth(-0.005) - 0.005).abs() < 1.0e-6,
        "{}",
        at_depth(-0.005)
    );
}

#[test]
fn a_dented_corner_reads_outside_where_the_corner_used_to_be() {
    // Pulling one corner in leaves the cube closed and makes it concave
    // there, so the sample below has a vertex as its closest feature — the
    // branch that reads the angle-weighted vertex pseudo-normal. It is
    // outside on an argument that needs no geometry: every vertex of the
    // dented cube has `x + y + z` at most 0.57 of a side, and the sample has
    // 0.585, so it is outside the convex hull and therefore outside the
    // body.
    let (mut positions, indices) = cube();
    for c in 0..3 {
        positions[6 * 3 + c] = 0.95 * SIDE;
    }
    let field = mesh_sdf(&positions, &indices, CELL, BAND).expect("a dent leaves it closed");
    let lattice = Lattice::around(&positions, CELL, BAND);
    let q = lattice.point(23, 23, 23);
    let got = f64::from(field.data[lattice.index(23, 23, 23)]);
    let truth = brute::distance(&positions, &indices, q);
    assert!(got > 0.0, "{q:?} reads {got}, and it is outside");
    assert!((got - truth).abs() < 1.0e-6, "{got} against {truth}");
}

#[test]
fn every_sample_is_its_true_distance_in_the_band_and_saturated_outside_it() {
    let (positions, indices) = cube();
    let field = mesh_sdf(&positions, &indices, CELL, BAND).expect("the cube is closed");
    let lattice = Lattice::around(&positions, CELL, BAND);
    let side = f64::from(SIDE);
    for k in 0..lattice.dims[2] {
        for j in 0..lattice.dims[1] {
            for i in 0..lattice.dims[0] {
                let q = lattice.point(i, j, k);
                let truth = brute::distance(&positions, &indices, q);
                let got = f64::from(field.data[lattice.index(i, j, k)]);
                let expected = if truth <= BAND { truth } else { BAND };
                assert!(
                    (got.abs() - expected).abs() < 1.0e-6,
                    "{q:?} reads {got}, brute force says {truth}"
                );
                if truth > 1.0e-9 {
                    let inside = q.iter().all(|&c| c > 0.0 && c < side);
                    assert_eq!(got < 0.0, inside, "{q:?} reads {got}");
                }
            }
        }
    }
}
