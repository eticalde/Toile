use toile_anny::Station;
use toile_anny::phenotype::age_param_from_years;

use super::anny::station_centroid;
use super::{BAND, CELL, CORNERS, FACES, bits, brute};
use crate::body::bake::{self, Lattice, inspect, mesh_sdf, sdf};
use crate::body::{NO_LEVERS, Phenotype, body_mesh};

/// The first box's far corner, in metres. Its near corner is the origin.
///
/// Two millimetres short of a round figure, and [`SECOND`] two millimetres
/// off one, so that no sample sits at exactly the band's reach from a face
/// and the test never has to ask which side of that edge a rounding fell.
const FIRST: f32 = 0.198;

/// The second box's near and far corners: pushed halfway through the first
/// along x, so each buries one face inside the other.
const SECOND: [[f32; 3]; 2] = [[0.102, 0.032, 0.032], [0.302, 0.232, 0.232]];

/// How close to the band's reach a sample is left unread, in metres.
const EDGE: f64 = 1.0e-6;

/// Two closed boxes in one mesh, the second passing through the first.
fn two_boxes() -> (Vec<f32>, Vec<u32>) {
    let mut positions: Vec<f32> = CORNERS.iter().flatten().map(|c| c * FIRST).collect();
    for corner in CORNERS {
        for c in 0..3 {
            positions.push(SECOND[0][c] + corner[c] * (SECOND[1][c] - SECOND[0][c]));
        }
    }
    let indices = FACES
        .iter()
        .flatten()
        .copied()
        .chain(FACES.iter().flatten().map(|i| i + 8))
        .collect();
    (positions, indices)
}

/// Whether a point is in either box.
fn in_a_box(q: [f64; 3]) -> bool {
    let first = q.iter().all(|&c| c > 0.0 && c < f64::from(FIRST));
    let second = (0..3).all(|c| q[c] > f64::from(SECOND[0][c]) && q[c] < f64::from(SECOND[1][c]));
    first || second
}

/// The core of each box is walled in by its own skin, which reads flesh,
/// and by the face the other box buries in it, which reads air. Settled by
/// whichever reading a scan comes to first, a slab of each core is air.
#[test]
fn a_core_walled_in_by_a_buried_face_bakes_as_flesh() {
    let (positions, indices) = two_boxes();
    assert!(
        inspect(&positions, &indices).earns_the_sign(),
        "counting half-edges cannot see one box inside the other"
    );
    let field = mesh_sdf(&positions, &indices, CELL, BAND).expect("both boxes are closed");
    let lattice = Lattice::around(&positions, CELL, BAND);
    let (mut flesh, mut air, mut buried) = (0, 0, 0);
    for k in 0..lattice.dims[2] {
        for j in 0..lattice.dims[1] {
            for i in 0..lattice.dims[0] {
                let q = lattice.point(i, j, k);
                let got = field.data[lattice.index(i, j, k)];
                let truth = brute::distance(&positions, &indices, q);
                if truth < BAND - EDGE {
                    buried += usize::from(truth > EDGE && (got < 0.0) != in_a_box(q));
                } else if truth > BAND + EDGE {
                    let side = if in_a_box(q) { -BAND } else { BAND } as f32;
                    assert_eq!(bits(got), bits(side), "{q:?} reads {got}");
                    flesh += usize::from(in_a_box(q));
                    air += usize::from(!in_a_box(q));
                }
            }
        }
    }
    assert!(flesh > 1000 && air > 1000, "{flesh} of flesh, {air} of air");
    // The fill does not mend the band itself. Beside a buried face the
    // nearest sheet is not the one that bounds the sample, and those samples
    // still read air: only a sign that counts how often the skin wraps a
    // point, rather than asking the nearest piece of it, would turn them.
    assert_eq!(buried, 1924, "band samples of flesh that read air");
}

/// The heavy muscular male, and the same build at the old end of the age
/// slider with the proportions lever pulled: two bodies the controls reach
/// whose buttocks pass through each other.
fn crossed_bodies() -> [Phenotype; 2] {
    let heavy = Phenotype {
        gender: 0.0,
        muscle: 1.0,
        weight: 1.0,
        ..Phenotype::default()
    };
    let old = Phenotype {
        age: age_param_from_years(100.0),
        proportions: 1.0,
        ..heavy
    };
    [heavy, old]
}

/// A build with its assertions on bakes a crossed body: the app's own does,
/// on the oven's thread, where a panic leaves the body without a field for
/// good. At twice the cell, which makes it affordable unoptimized and leaves
/// the older body's pelvis disputed all the same.
#[test]
fn a_crossed_body_bakes_with_the_assertions_on() {
    let [_, old] = crossed_bodies();
    let mesh = body_mesh(&old, &NO_LEVERS);
    let field = mesh_sdf(&mesh.positions, &mesh.indices, 2.0 * bake::CELL, bake::BAND)
        .expect("closed and orientable, crossed or not");
    let core = station_centroid(&mesh, Station::Waist);
    assert_eq!(
        bits(field.sample(core[0], core[1], core[2])),
        bits(-(bake::BAND as f32)),
        "the core of the waist is one region with the disputed pelvis"
    );
}

/// Both bodies at the cell the app bakes them with, their saturated samples
/// held against the winding number: every one where the buttocks meet, and
/// one in a few thousand over the rest of the box.
#[test]
#[ignore = "release-only: this bakes millions of voxels"]
fn a_body_whose_buttocks_cross_bakes_and_its_flesh_reads_flesh() {
    // Holds every sample that settling a doubtful region by scan order
    // leaves as air on either body, and is read in full rather than by stride.
    const CLEFT: [[f64; 2]; 3] = [[-0.05, 0.05], [0.14, 0.29], [-0.15, -0.09]];
    const STRIDE: usize = 3_671;
    // How far round the cleft a triangle is looked at for a crossing.
    const REACH: f64 = 0.1;

    for phenotype in crossed_bodies() {
        let mesh = body_mesh(&phenotype, &NO_LEVERS);
        let pelvis: Vec<[u32; 3]> = mesh
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .filter(|t| {
                t.iter().all(|&v| {
                    let p = &mesh.positions[v as usize * 3..v as usize * 3 + 3];
                    (0..3).all(|c| {
                        let x = f64::from(p[c]);
                        x > CLEFT[c][0] - REACH && x < CLEFT[c][1] + REACH
                    })
                })
            })
            .copied()
            .collect();
        let crossings = brute::crossings(&mesh.positions, &pelvis);
        assert!(crossings > 0, "this body no longer crosses itself there");

        let field = sdf(&mesh).expect("closed and orientable, crossed or not");
        let lattice = Lattice::around(&mesh.positions, bake::CELL, bake::BAND);
        let [nx, ny, _] = lattice.dims;
        let (mut read, mut flesh) = (0, 0);
        for slot in 0..field.data.len() {
            let got = field.data[slot];
            let q = lattice.point(slot % nx, (slot / nx) % ny, slot / (nx * ny));
            let near = (0..3).all(|c| q[c] >= CLEFT[c][0] && q[c] <= CLEFT[c][1]);
            if f64::from(got.abs()) < bake::BAND || !(near || slot % STRIDE == 0) {
                continue;
            }
            let wraps = brute::winding(&mesh.positions, &mesh.indices, q);
            assert_eq!(got < 0.0, wraps > 0.5, "{q:?} reads {got}, wrapped {wraps}");
            read += 1;
            flesh += usize::from(near && got < 0.0);
        }
        println!(
            "{crossings} crossings; {read} saturated samples read, {flesh} of flesh at the cleft"
        );
        assert!(
            read > 2000 && flesh > 100,
            "{read} read, {flesh} at the cleft"
        );
    }
}
