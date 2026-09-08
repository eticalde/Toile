#![allow(
    clippy::float_cmp,
    clippy::cast_precision_loss,
    reason = "structural invariants over a mesh built from f64 literals"
)]

mod anatomy;
mod derive;

use crate::landmarks::landmarks;
use crate::loft::vertex_normals;
use crate::mesh::{BodyMesh, body_mesh, parts};
use crate::params::{BodyMeasures, BodyRes};
use crate::parts::Part;
use crate::ring::{Shape, from_girth, perimeter, unit_dirs};

/// The default resolution's radial segments.
const SEG: usize = 32;
/// Rings from one station to the next at the default resolution.
const SPAN: usize = 5;

fn reference() -> BodyMesh {
    body_mesh(&BodyMeasures::default(), BodyRes::default())
}

/// The five parts, uncentred: y runs from the ankle joint.
fn parts_of(m: &BodyMeasures) -> [Part; 5] {
    parts(m, BodyRes::default())
}

/// Ring `index` of a part: rings are contiguous blocks of `seg + 1`
/// vertices; the cap centres trail them. Station `i` of a tube is ring
/// `i * SPAN`.
fn ring(part: &Part, index: usize) -> &[[f32; 3]] {
    let width = SEG + 1;
    &part.0.as_chunks::<3>().0[index * width..(index + 1) * width]
}

fn perimeter_of(ring: &[[f32; 3]]) -> f64 {
    ring.windows(2)
        .map(|w| {
            let d = [w[1][0] - w[0][0], w[1][1] - w[0][1], w[1][2] - w[0][2]];
            f64::from(d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
        })
        .sum()
}

/// `(min_x, max_x, min_z, max_z)` of a ring.
fn extent(ring: &[[f32; 3]]) -> (f64, f64, f64, f64) {
    ring.iter().fold(
        (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ),
        |e, p| {
            let (x, z) = (f64::from(p[0]), f64::from(p[2]));
            (e.0.min(x), e.1.max(x), e.2.min(z), e.3.max(z))
        },
    )
}

fn centroid(ring: &[[f32; 3]]) -> [f64; 3] {
    let mut c = [0.0; 3];
    for p in &ring[..SEG] {
        for (k, v) in c.iter_mut().enumerate() {
            *v += f64::from(p[k]);
        }
    }
    c.map(|v| v / SEG as f64)
}

#[test]
fn the_mesh_is_well_formed() {
    let m = reference();
    let n = m.vertex_count();
    assert!(n > 0);
    assert_eq!(m.positions.len(), m.normals.len());
    assert_eq!(m.indices.len() % 3, 0);
    assert!(m.indices.iter().all(|&i| (i as usize) < n));
    assert!(m.positions.iter().all(|f| f.is_finite()));
    assert!(m.normals.iter().all(|f| f.is_finite()));
}

#[test]
fn every_normal_is_unit_length() {
    let m = reference();
    for n in m.normals.as_chunks::<3>().0 {
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        assert!((len - 1.0).abs() < 1.0e-4, "normal length {len}");
    }
}

#[test]
fn topology_is_fixed_across_measurements() {
    let a = body_mesh(&BodyMeasures::default(), BodyRes::default());
    let wide = BodyMeasures {
        waist: 110.0,
        hip: 130.0,
        bust: 120.0,
        underbust: 70.0,
        upper_chest: 110.0,
        shoulder_width: 60.0,
        arm_length: 80.0,
        upper_arm: 45.0,
        wrist: 25.0,
        neck: 50.0,
        head: 65.0,
        back_length: 55.0,
        ..BodyMeasures::default()
    };
    let b = body_mesh(&wide, BodyRes::default());
    assert_eq!(a.positions.len(), b.positions.len());
    assert_eq!(a.indices, b.indices);
}

#[test]
fn landmarks_are_strictly_ordered() {
    let s = landmarks(&BodyMeasures::default()).stack();
    assert!(0.0 < s[0]);
    for w in s.windows(2) {
        assert!(w[0] < w[1], "{w:?}");
    }
}

#[test]
fn bad_input_keeps_the_landmarks_ordered() {
    // Nonsense measures must still loft: the clamps hold the order.
    let junk = BodyMeasures {
        height: 1.0,
        rise: 900.0,
        hip_drop: 900.0,
        outseam: 5.0,
        back_length: 900.0,
        shoulder_width: 0.0,
        neck: 300.0,
        bust: 0.0,
        head: 1.0,
        arm_length: -5.0,
        upper_arm: 200.0,
        ..BodyMeasures::default()
    };
    let s = landmarks(&junk).stack();
    assert!(0.0 < s[0]);
    for w in s.windows(2) {
        assert!(w[0] < w[1], "{w:?}");
    }
    let mesh = body_mesh(&junk, BodyRes::default());
    assert!(mesh.vertex_count() > 0);
    assert!(mesh.positions.iter().all(|f| f.is_finite()));
}

#[test]
fn the_height_spans_the_measurement() {
    let m = reference();
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for y in m.positions.iter().skip(1).step_by(3) {
        lo = lo.min(*y);
        hi = hi.max(*y);
    }
    // The ankle joint sits 7 cm above the floor, so the modelled span is the
    // stature less that offset: ankle ring at 0, skull apex at the crown (the
    // wrists end at crotch height).
    let span = f64::from(hi - lo);
    assert!((span - 1.71).abs() < 0.02, "span {span}");
}

#[test]
fn a_wider_waist_widens_the_waist_ring() {
    // Measured on the trunk part's own waist ring: an arm ring crosses the
    // waist height too.
    let width = |waist: f64| {
        let m = BodyMeasures {
            waist,
            ..BodyMeasures::default()
        };
        let [trunk, ..] = parts_of(&m);
        ring(&trunk, 2 * SPAN)
            .iter()
            .fold(0.0f32, |w, p| w.max(p[0].abs()))
    };
    assert!(width(94.0) > width(74.0));
}

#[test]
fn each_landmark_ring_carries_its_girth() {
    // from_girth honours the tape over split depths and blended roundness: a
    // ring built for a girth has a chord-sum perimeter equal to it.
    let dirs = unit_dirs(32);
    let shapes = [
        (0.76, 0.66, 0.55, 0.85),  // waist
        (0.66, 0.852, 0.55, 0.45), // hip
        (1.02, 0.72, 0.23, 0.80),  // bust, delta 16
        (0.95, 1.15, 0.10, 0.10),  // calf
    ];
    for (rho_front, rho_back, round_front, round_back) in shapes {
        let s = Shape {
            rho_front,
            rho_back,
            round_front,
            round_back,
        };
        for girth in [58.0, 84.0, 98.0] {
            let r = from_girth(girth, s, 0.0, 0.0, 0.0, &dirs);
            let p = perimeter(&r, &dirs);
            assert!((p - girth / 100.0).abs() < 1.0e-6, "girth {girth} -> {p}");
        }
    }
}

#[test]
fn normals_point_outward() {
    // Per part, radial = vertex − its ring's centroid, which holds for tilted
    // arm rings and the head's forward carriage alike.
    for part in &parts_of(&BodyMeasures::default()) {
        let normals = vertex_normals(&part.0, &part.1);
        let verts = part.0.as_chunks::<3>().0;
        let rings = verts.len() / (SEG + 1);
        let (mut out, mut total) = (0usize, 0usize);
        for r in 0..rings {
            let c = centroid(ring(part, r));
            for (p, n) in ring(part, r)
                .iter()
                .zip(&normals.as_chunks::<3>().0[r * (SEG + 1)..])
            {
                let d = [
                    f64::from(p[0]) - c[0],
                    f64::from(p[1]) - c[1],
                    f64::from(p[2]) - c[2],
                ];
                if (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() < 0.02 {
                    continue; // skip apex-hugging rings
                }
                total += 1;
                if d[0] * f64::from(n[0]) + d[1] * f64::from(n[1]) + d[2] * f64::from(n[2]) > 0.0 {
                    out += 1;
                }
            }
        }
        assert!(out as f64 > 0.9 * total as f64, "{out}/{total} outward");
    }
}
