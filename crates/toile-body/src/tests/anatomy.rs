use super::{SPAN, centroid, extent, parts_of, perimeter_of, ring};
use crate::landmarks::landmarks;
use crate::params::{BodyMeasures, BodyRes};
use crate::parts::limb::{POSE_COS, POSE_SIN, joint};
use crate::parts::trunk::stations;
use crate::parts::{Ctx, Side};
use crate::profile::interpolate;
use crate::ring::unit_dirs;

fn with_ctx<T>(m: &BodyMeasures, f: impl FnOnce(&Ctx) -> T) -> T {
    let dirs = unit_dirs(32);
    let lm = landmarks(m);
    f(&Ctx {
        m,
        res: BodyRes::default(),
        lm: &lm,
        dirs: &dirs,
    })
}

#[test]
fn every_upper_station_carries_its_girth() {
    let m = BodyMeasures::default();
    let [trunk, _, _, _, arm] = parts_of(&m);
    let expect = [
        (&trunk, 3, m.underbust),
        (&trunk, 4, m.bust),
        (&trunk, 5, m.upper_chest),
        (&trunk, 7, m.neck),
        (&trunk, 11, m.head),
        (&arm, 0, m.wrist),
        (&arm, 3, m.upper_arm),
    ];
    for (part, station, girth) in expect {
        let p = perimeter_of(ring(part, station * SPAN));
        assert!(
            (p - girth / 100.0).abs() < 1.0e-4,
            "station {station}: {p} vs {girth}"
        );
    }
}

#[test]
fn the_shoulders_span_the_shoulder_width() {
    let m = BodyMeasures::default();
    let [trunk, ..] = parts_of(&m);
    let (lo, hi, ..) = extent(ring(&trunk, 6 * SPAN));
    assert!(
        (hi - lo - m.shoulder_width / 100.0).abs() < 1.0e-4,
        "{}",
        hi - lo
    );
    let (armpit_lo, armpit_hi, ..) = extent(ring(&trunk, 5 * SPAN));
    assert!(
        hi - lo > armpit_hi - armpit_lo,
        "the shoulders widen the trunk top"
    );
}

#[test]
fn the_head_height_follows_the_stature() {
    let m = BodyMeasures::default();
    let lm = landmarks(&m);
    assert!((lm.crown - lm.jaw - 0.115 * 1.78).abs() < 1.0e-9);
    let [trunk, ..] = parts_of(&m);
    let (lo, hi, back, front) = extent(ring(&trunk, 11 * SPAN));
    assert!((hi - lo - 0.156).abs() < 0.005, "head width {}", hi - lo);
    assert!(
        (front - back - 0.196).abs() < 0.005,
        "head depth {}",
        front - back
    );
    let top = trunk
        .0
        .iter()
        .skip(1)
        .step_by(3)
        .fold(f32::NEG_INFINITY, |a, &y| a.max(y));
    assert!(
        (f64::from(top) - lm.crown).abs() < 1.0e-6,
        "apex {top} vs crown {}",
        lm.crown
    );
}

#[test]
fn the_pose_literals_are_a_unit_pair() {
    assert!((POSE_COS * POSE_COS + POSE_SIN * POSE_SIN - 1.0).abs() < 1.0e-15);
}

#[test]
fn the_arm_reaches_the_wrist_in_a_pose() {
    let m = BodyMeasures::default();
    let lm = landmarks(&m);
    let [_, _, _, left, right] = parts_of(&m);
    let j = with_ctx(&m, |c| joint(c, Side::Right));
    let len = m.arm_length / 100.0;
    let w = centroid(ring(&right, 0));
    let want = [j[0] + len * POSE_SIN, j[1] - len * POSE_COS, j[2]];
    for k in 0..3 {
        assert!((w[k] - want[k]).abs() < 1.0e-3, "wrist {w:?} vs {want:?}");
    }
    // The tilt read off the mesh is the pose tangent — no trig in the test.
    let slope = (w[0] - j[0]) / (j[1] - w[1]);
    assert!(
        (slope - POSE_SIN / POSE_COS).abs() < 1.0e-3,
        "slope {slope}"
    );
    let wl = centroid(ring(&left, 0));
    assert!((wl[0] + w[0]).abs() < 1.0e-5 && (wl[1] - w[1]).abs() < 1.0e-5);
    let lowest = right
        .0
        .iter()
        .skip(1)
        .step_by(3)
        .fold(f32::INFINITY, |a, &y| a.min(y));
    assert!(f64::from(lowest) > lm.crotch, "lowest arm vertex {lowest}");
}

/// The widest trunk vertex within 2 cm of `y`, optionally restricted to a
/// band around the arm's carriage plane.
fn trunk_width_at(trunk: &[[f32; 3]], y: f64, z_band: Option<f64>) -> f64 {
    trunk
        .iter()
        .filter(|p| (f64::from(p[1]) - y).abs() < 0.02)
        .filter(|p| z_band.is_none_or(|b| (f64::from(p[2]) - 0.01).abs() < b))
        .fold(0.0f64, |w, p| w.max(f64::from(p[0].abs())))
}

#[test]
fn the_arms_clear_the_trunk_below_the_armpit() {
    let m = BodyMeasures::default();
    let lm = landmarks(&m);
    let [trunk, _, _, _, right] = parts_of(&m);
    let trunk = trunk.0.as_chunks::<3>().0;
    for p in right.0.as_chunks::<3>().0 {
        let y = f64::from(p[1]);
        if y <= lm.armpit - 0.04 {
            let w = trunk_width_at(trunk, y, None);
            assert!(
                f64::from(p[0].abs()) > w,
                "arm vertex {p:?} inside the trunk ({w})"
            );
        }
    }
}

#[test]
fn the_arm_top_hides_inside_the_shoulder() {
    let m = BodyMeasures::default();
    let [trunk, _, _, _, right] = parts_of(&m);
    let trunk = trunk.0.as_chunks::<3>().0;
    let verts = right.0.as_chunks::<3>().0;
    let cap = verts[verts.len() - 1]; // the joint cap centre trails the wrist's
    for p in ring(&right, 4 * SPAN).iter().chain([&cap]) {
        let w = trunk_width_at(trunk, f64::from(p[1]), Some(0.078));
        assert!(
            f64::from(p[0].abs()) < w,
            "deltoid vertex {p:?} outside the trunk ({w})"
        );
    }
}

#[test]
fn the_thigh_top_hides_inside_the_pelvis() {
    let m = BodyMeasures::default();
    let [trunk, left, right, ..] = parts_of(&m);
    let (_, a_crotch, ..) = extent(ring(&trunk, 0));
    for leg in [&left, &right] {
        for p in ring(leg, 3 * SPAN) {
            let (x, z) = (f64::from(p[0].abs()), f64::from(p[2]));
            assert!(x <= a_crotch + 1.0e-3, "thigh x {x} vs pelvis {a_crotch}");
            assert!((-0.135..=0.102).contains(&z), "thigh z {z}");
        }
    }
}

/// `(max_z of the bust ring − max_z of the waist ring, min_z of the bust
/// ring)`.
fn bust_lead(m: &BodyMeasures) -> (f64, f64) {
    let [trunk, ..] = parts_of(m);
    let (.., bust_back, bust_front) = extent(ring(&trunk, 4 * SPAN));
    let (.., waist_front) = extent(ring(&trunk, 2 * SPAN));
    (bust_front - waist_front, bust_back)
}

#[test]
fn the_bust_leads_the_waist_when_the_numbers_say_so() {
    let reference = BodyMeasures::default();
    let (lead, back) = bust_lead(&reference);
    assert!(lead > 0.02, "reference lead {lead}");
    let full = BodyMeasures {
        bust: 92.0,
        underbust: 76.0,
        upper_chest: 84.0,
        waist: 70.0,
        hip: 96.0,
        ..reference
    };
    let (lead_full, back_full) = bust_lead(&full);
    assert!(lead_full > 0.05, "full lead {lead_full}");
    let flat = BodyMeasures {
        bust: 88.0,
        underbust: 88.0,
        upper_chest: 88.0,
        ..reference
    };
    let (lead_flat, back_flat) = bust_lead(&flat);
    assert!(lead_flat < 0.015, "flat lead {lead_flat}");
    // The back does not follow the bust: the numbers move the front only.
    assert!((back_full - back).abs() < 0.02 && (back_flat - back).abs() < 0.02);
}

#[test]
fn the_seat_sits_behind_and_the_waist_back_is_flat() {
    let seat = |m: &BodyMeasures| {
        let [trunk, ..] = parts_of(m);
        let (.., hip_back, _) = extent(ring(&trunk, SPAN));
        let (.., waist_back, waist_front) = extent(ring(&trunk, 2 * SPAN));
        (hip_back, waist_back, waist_front)
    };
    let (hip_back, waist_back, waist_front) = seat(&BodyMeasures::default());
    assert!(
        hip_back < waist_back - 0.03,
        "seat {hip_back} vs waist {waist_back}"
    );
    assert!(
        waist_back.abs() < waist_front,
        "the waist back is flatter than its front"
    );
    let (deep, ..) = seat(&BodyMeasures {
        hip: 100.0,
        waist: 70.0,
        ..BodyMeasures::default()
    });
    assert!(
        deep < hip_back,
        "a bigger drop pushes the seat back: {deep} vs {hip_back}"
    );
}

#[test]
fn no_profile_overshoots_its_stations() {
    let m = BodyMeasures::default();
    let secs = with_ctx(&m, stations);
    let rings = interpolate(&secs, 4);
    assert_eq!(rings.len(), (secs.len() - 1) * 5 + 1);
    for (i, w) in secs.windows(2).enumerate() {
        for r in &rings[i * 5..(i + 1) * 5] {
            for (v, lo, hi) in [
                (r.a, w[0].a, w[1].a),
                (r.depth_front, w[0].depth_front, w[1].depth_front),
                (r.depth_back, w[0].depth_back, w[1].depth_back),
            ] {
                assert!(
                    v >= lo.min(hi) - 1.0e-9 && v <= lo.max(hi) + 1.0e-9,
                    "span {i}: {v} outside [{lo}, {hi}]"
                );
            }
            assert!((0.0..=1.0).contains(&r.round_front) && (0.0..=1.0).contains(&r.round_back));
        }
    }
}
