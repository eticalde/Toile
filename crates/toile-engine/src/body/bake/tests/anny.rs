use std::sync::OnceLock;
use std::time::Instant;

use toile_anny::{BodyMesh, Station};
use toile_sim::xpbd::SdfGrid;

use super::{bits, brute};
use crate::body::bake::{BAND, CELL, Lattice, inspect, sdf};
use crate::body::{NO_LEVERS, Phenotype, body_mesh};
use crate::golden::REFERENCE;

/// How far under the skin the field is read, in metres: a cell and a half
/// in, so the answer is interpolated between voxels rather than read off
/// one, and still far enough out that the band is live there.
const SKIN_DEPTH: f32 = 0.008;

/// The stations whose vertices sit on the trunk, where the body is thicker
/// than [`SKIN_DEPTH`] by an order of magnitude whatever the phenotype.
const TRUNK: [Station; 4] = [
    Station::Waist,
    Station::Hip,
    Station::Bust,
    Station::Underbust,
];

/// Anny's own neutral adult with no lever pulled, and its baked field.
///
/// Both are built once however many of the tests below run: the bake is the
/// expensive thing on this page, and it is the same bake for all of them.
fn baked() -> &'static (BodyMesh, SdfGrid) {
    static ONCE: OnceLock<(BodyMesh, SdfGrid)> = OnceLock::new();
    ONCE.get_or_init(|| {
        let mesh = body_mesh(&Phenotype::default(), &NO_LEVERS);
        let started = Instant::now();
        let field = sdf(&mesh).expect("the Anny body is closed and orientable");
        println!(
            "bake: {:?} voxels in {:.2} s, {:.1} MB",
            field.dims,
            started.elapsed().as_secs_f64(),
            field.data.len() as f64 * 4.0 / 1.0e6
        );
        (mesh, field)
    })
}

#[test]
fn the_anny_body_is_the_closed_orientable_mesh_the_sign_needs() {
    let mesh = body_mesh(&Phenotype::default(), &NO_LEVERS);
    let m = inspect(&mesh.positions, &mesh.indices);
    println!("{m:?}");
    assert_eq!((m.vertices, m.triangles, m.edges), (13_380, 26_756, 40_134));
    assert_eq!(m.boundary_edges, 0, "no hole");
    assert_eq!(m.nonmanifold_edges, 0, "no edge with three faces on it");
    assert_eq!(m.inconsistent_edges, 0, "no flipped face");
    assert_eq!(m.degenerate_triangles, 0, "every face has a normal");
    assert_eq!(m.euler, 2, "one closed surface of genus 0");
    assert!((40.0..160.0).contains(&m.litres), "{} litres", m.litres);
    assert!(m.earns_the_sign());
}

#[test]
#[ignore = "release-only: this bakes millions of voxels"]
fn the_grid_wraps_the_body_with_a_clear_band_around_it() {
    let (mesh, field) = baked();
    let lattice = Lattice::around(&mesh.positions, CELL, BAND);
    assert_eq!(field.dims, lattice.dims);
    assert_eq!(bits(field.cell), bits(CELL as f32));
    assert!(
        field.dims[1] > field.dims[0] && field.dims[1] > field.dims[2],
        "a standing body is taller than it is wide: {:?}",
        field.dims
    );
    // The box is as wide as the arms reach rather than as the shoulders do,
    // so a good part of it is the air at the armpits. That is what one
    // axis-aligned box costs, and it is paid once per body.
    let voxels = field.data.len();
    assert!(
        voxels < 16_000_000,
        "{voxels} voxels is more than a body's box needs"
    );
    for (c, &n) in field.dims.iter().enumerate() {
        let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
        for p in mesh.positions.as_chunks::<3>().0 {
            lo = lo.min(p[c]);
            hi = hi.max(p[c]);
        }
        let last = field.origin[c] + (n - 1) as f32 * field.cell;
        let pad = (BAND + CELL) as f32;
        assert!(field.origin[c] <= lo - pad, "axis {c} starts too late");
        assert!(last >= hi + pad, "axis {c} ends too early");
    }
}

#[test]
#[ignore = "release-only: this bakes millions of voxels"]
fn the_sign_is_right_in_the_abdomen_across_the_room_and_under_the_skin() {
    let (mesh, field) = baked();
    let core = station_centroid(mesh, Station::Waist);
    let inside = field.sample(core[0], core[1], core[2]);
    assert_eq!(
        bits(inside),
        bits(-(BAND as f32)),
        "the centroid of the waist ring is deeper in than the band reaches, \
         and reads {inside}"
    );
    let across = field.sample(3.0, 1.0, 3.0);
    assert_eq!(
        bits(across),
        bits(BAND as f32),
        "three metres away: {across}"
    );

    let mut read = 0;
    for v in 0..mesh.vertex_count() {
        if !TRUNK.iter().any(|s| s.tag() == mesh.stations[v]) {
            continue;
        }
        let step = |depth: f32| {
            let p = [0, 1, 2].map(|c| mesh.positions[v * 3 + c] + depth * mesh.normals[v * 3 + c]);
            field.sample(p[0], p[1], p[2])
        };
        let under = step(-SKIN_DEPTH);
        assert!(
            (-2.0 * BAND as f32..0.0).contains(&under),
            "{SKIN_DEPTH} m under the skin at trunk vertex {v} reads {under}"
        );
        assert!(step(SKIN_DEPTH) > 0.0, "off the skin at vertex {v}");
        read += 1;
    }
    assert!(read > 500, "only {read} trunk vertices were read");
}

#[test]
#[ignore = "release-only: this bakes millions of voxels"]
fn the_band_agrees_with_a_brute_force_distance_over_the_whole_body() {
    // A prime stride, so the samples are not all on one plane of the grid
    // the way a round one would put them.
    const STRIDE: usize = 3_671;

    let (mesh, field) = baked();
    let lattice = Lattice::around(&mesh.positions, CELL, BAND);
    let (mut read, mut worst) = (0, 0.0f64);
    for slot in (0..field.data.len()).step_by(STRIDE) {
        let got = f64::from(field.data[slot]);
        if got.abs() >= BAND {
            continue;
        }
        let [nx, ny, _] = lattice.dims;
        let q = lattice.point(slot % nx, (slot / nx) % ny, slot / (nx * ny));
        let truth = brute::distance(&mesh.positions, &mesh.indices, q);
        worst = worst.max((got.abs() - truth).abs());
        read += 1;
    }
    println!("brute force: {read} samples in the band, worst {worst:.3e} m");
    assert!(
        read > 100,
        "only {read} of the samples were inside the band"
    );
    // A micron. The two routes are both f64 over the same vertices, so what
    // is left is the narrowing to f32 the grid stores, which at 2.5 cm is
    // two nanometres.
    assert!(worst < 1.0e-6, "worst disagreement {worst} m over {read}");
}

/// The default body's soles bake four voxels of solid flesh as air, and the
/// body the goldens are taken against bakes none.
///
/// Bærentzen–Aanæs is exact for a closed orientable surface that does not
/// pass through itself, and [`inspect`] counts half-edges, which that third
/// condition is invisible to. Where two sheets cross, the sheet nearest a
/// point is not the one that bounds it, and the sign follows the nearest —
/// so what is asked here is the winding number, which counts how often the
/// skin wraps the point rather than which piece of it is closest.
///
/// Whoever moves these numbers has a choice to make rather than a constant to
/// re-pin: mend the soles where the mesh is baked, teach the bake to see a
/// crossing, or write down that four voxels buried in a sole are a price
/// worth paying.
#[test]
#[ignore = "release-only: this bakes millions of voxels"]
fn the_default_bodys_soles_bake_four_voxels_of_flesh_as_air() {
    // How deep a slab off the underside of a body counts as its soles.
    const SOLES: f64 = 0.03;
    // The voxels the default body loses, in the grid its own box makes.
    const LOST: [[usize; 3]; 4] = [[76, 9, 60], [77, 9, 60], [169, 9, 60], [170, 9, 60]];

    let (mesh, field) = baked();
    let lattice = Lattice::around(&mesh.positions, CELL, BAND);
    for lost in LOST {
        let [i, j, k] = lost;
        let got = field.data[lattice.index(i, j, k)];
        assert!(
            got > 0.0,
            "{lost:?} reads {got}, so the field calls it flesh"
        );
        let wraps = brute::winding(&mesh.positions, &mesh.indices, lattice.point(i, j, k));
        assert!(
            (wraps - 1.0).abs() < 1.0e-6,
            "{lost:?} is wrapped {wraps} times, so it is not solid sole"
        );
    }

    let reference = body_mesh(&REFERENCE, &NO_LEVERS);
    assert_eq!(
        brute::crossings(&mesh.positions, &soles(mesh, SOLES)),
        52,
        "pairs of sole triangles the default body passes through itself"
    );
    assert_eq!(
        brute::crossings(&reference.positions, &soles(&reference, SOLES)),
        0,
        "and the body the goldens are taken against crosses none, which is \
         why its pinned field is clean and this one is not"
    );
}

/// The triangles with a vertex within `depth` of the body's lowest point.
fn soles(mesh: &BodyMesh, depth: f64) -> Vec<[u32; 3]> {
    let floor = mesh
        .positions
        .as_chunks::<3>()
        .0
        .iter()
        .map(|p| f64::from(p[1]))
        .fold(f64::INFINITY, f64::min);
    mesh.indices
        .as_chunks::<3>()
        .0
        .iter()
        .filter(|t| {
            t.iter()
                .any(|&v| f64::from(mesh.positions[v as usize * 3 + 1]) <= floor + depth)
        })
        .copied()
        .collect()
}

/// The average of every vertex carrying one station tag: a point inside the
/// body, put there by the anatomy rather than by a guess at a coordinate.
fn station_centroid(mesh: &BodyMesh, station: Station) -> [f32; 3] {
    let mut sum = [0.0f64; 3];
    let mut n = 0.0;
    for v in 0..mesh.vertex_count() {
        if mesh.stations[v] == station.tag() {
            for (s, p) in sum.iter_mut().zip(&mesh.positions[v * 3..v * 3 + 3]) {
                *s += f64::from(*p);
            }
            n += 1.0;
        }
    }
    [0, 1, 2].map(|c| (sum[c] / n) as f32)
}
