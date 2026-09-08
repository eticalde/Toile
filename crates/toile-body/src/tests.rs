#![allow(
    clippy::float_cmp,
    clippy::cast_precision_loss,
    reason = "structural invariants over a mesh built from f64 literals"
)]

use crate::loft::landmarks;
use crate::mesh::{BodyMesh, body_mesh};
use crate::params::{BodyMeasures, BodyRes};
use crate::ring::{Ring, axes_for_girth, point, unit_dirs};

fn reference() -> BodyMesh {
    body_mesh(&BodyMeasures::default(), BodyRes::default())
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
        ..BodyMeasures::default()
    };
    let b = body_mesh(&wide, BodyRes::default());
    assert_eq!(a.positions.len(), b.positions.len());
    assert_eq!(a.indices, b.indices);
}

#[test]
fn landmarks_are_strictly_ordered() {
    let lm = landmarks(&BodyMeasures::default());
    assert!(0.0 < lm.knee);
    assert!(lm.knee < lm.crotch);
    assert!(lm.crotch < lm.hip);
    assert!(lm.hip < lm.waist);
    assert!(lm.waist < lm.crown);
}

#[test]
fn bad_input_keeps_the_landmarks_ordered() {
    // Nonsense measures must still loft: the clamps hold the order.
    let junk = BodyMeasures {
        height: 1.0,
        rise: 900.0,
        hip_drop: 900.0,
        outseam: 5.0,
        ..BodyMeasures::default()
    };
    let lm = landmarks(&junk);
    assert!(0.0 < lm.knee && lm.knee < lm.crotch);
    assert!(lm.crotch < lm.hip && lm.hip < lm.waist && lm.waist < lm.crown);
    assert!(body_mesh(&junk, BodyRes::default()).vertex_count() > 0);
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
    // stature less that offset.
    let span = f64::from(hi - lo);
    assert!((span - 1.71).abs() < 0.02, "span {span}");
}

#[test]
fn a_wider_waist_widens_the_waist_ring() {
    let widths = |waist: f64| {
        let m = BodyMeasures {
            waist,
            ..BodyMeasures::default()
        };
        let lm = landmarks(&m);
        let waist_y = (lm.waist - lm.crown * 0.5) as f32;
        let mesh = body_mesh(&m, BodyRes::default());
        let mut max_x = 0.0f32;
        for p in mesh.positions.as_chunks::<3>().0 {
            if (p[1] - waist_y).abs() < 1.0e-3 {
                max_x = max_x.max(p[0].abs());
            }
        }
        max_x
    };
    assert!(widths(94.0) > widths(74.0));
}

#[test]
fn each_landmark_ring_carries_its_girth() {
    // axes_for_girth honours the tape: a ring built from its axes has a
    // perimeter equal to the girth it was asked for.
    let dirs = unit_dirs(32);
    for girth in [58.0, 84.0, 98.0] {
        let (a, b) = axes_for_girth(girth, 0.78, 4.0, &dirs);
        let r = Ring {
            cx: 0.0,
            cz: 0.0,
            y: 0.0,
            a,
            b,
            n: 4.0,
        };
        let mut p = 0.0;
        for w in dirs.windows(2) {
            let x = point(&r, w[0]);
            let z = point(&r, w[1]);
            let (dx, dz) = (f64::from(z[0] - x[0]), f64::from(z[2] - x[2]));
            p += (dx * dx + dz * dz).sqrt();
        }
        assert!((p - girth / 100.0).abs() < 1.0e-6, "girth {girth} -> {p}");
    }
}

#[test]
fn normals_point_outward() {
    let m = BodyMeasures::default();
    let dirs = unit_dirs(32);
    let dx = crate::loft::hip_half_width(&m, &dirs);
    let mesh = body_mesh(&m, BodyRes::default());
    let (mut out, mut total) = (0usize, 0usize);
    for (p, n) in mesh
        .positions
        .as_chunks::<3>()
        .0
        .iter()
        .zip(mesh.normals.as_chunks::<3>().0)
    {
        // The nearest vertical axis is the trunk (x = 0) or a leg (x = ±dx).
        let cx = [-dx as f32, 0.0, dx as f32]
            .into_iter()
            .min_by(|a, b| (p[0] - a).abs().total_cmp(&(p[0] - b).abs()))
            .unwrap();
        let (rx, rz) = (p[0] - cx, p[2]);
        if (rx * rx + rz * rz).sqrt() < 0.02 {
            continue; // skip axis-hugging apex and cap vertices
        }
        total += 1;
        if rx * n[0] + rz * n[2] > 0.0 {
            out += 1;
        }
    }
    assert!(out as f64 > 0.9 * total as f64, "{out}/{total} outward");
}
