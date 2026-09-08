use crate::derive::PartialMeasures;
use crate::params::BodyMeasures;

#[test]
fn nothing_given_derives_the_reference_exactly() {
    assert_eq!(
        PartialMeasures::default().complete(),
        BodyMeasures::default()
    );
    let only_height = PartialMeasures {
        height: Some(178.0),
        ..PartialMeasures::default()
    };
    assert_eq!(only_height.complete(), BodyMeasures::default());
}

#[test]
fn a_shorter_person_gets_a_smaller_body() {
    let short = PartialMeasures {
        height: Some(160.0),
        ..PartialMeasures::default()
    }
    .complete();
    let r = BodyMeasures::default();
    let pairs = [
        (short.height, r.height),
        (short.waist, r.waist),
        (short.hip, r.hip),
        (short.thigh, r.thigh),
        (short.knee, r.knee),
        (short.ankle, r.ankle),
        (short.rise, r.rise),
        (short.outseam, r.outseam),
        (short.inseam, r.inseam),
        (short.hip_drop, r.hip_drop),
        (short.neck, r.neck),
        (short.bust, r.bust),
        (short.upper_chest, r.upper_chest),
        (short.underbust, r.underbust),
        (short.shoulder_width, r.shoulder_width),
        (short.arm_length, r.arm_length),
        (short.upper_arm, r.upper_arm),
        (short.wrist, r.wrist),
        (short.back_length, r.back_length),
        (short.head, r.head),
    ];
    for (s, r) in pairs {
        assert!(s < r, "{s} < {r}");
    }
}

#[test]
fn a_given_value_always_wins() {
    let m = PartialMeasures {
        waist: Some(70.0),
        ..PartialMeasures::default()
    }
    .complete();
    assert_eq!(m.waist, 70.0);
    assert_eq!(m.hip, 98.0);
}

#[test]
fn a_small_waist_under_a_bust_derives_prominence() {
    let m = PartialMeasures {
        height: Some(178.0),
        waist: Some(68.0),
        hip: Some(96.0),
        ..PartialMeasures::default()
    }
    .complete();
    assert!((m.bust - 87.0).abs() < 1.0e-9, "bust {}", m.bust);
    assert!(
        (m.underbust - 75.5).abs() < 1.0e-9,
        "underbust {}",
        m.underbust
    );
    assert!(m.bust - m.upper_chest > 2.5);

    let hung = PartialMeasures {
        underbust: Some(80.0),
        ..PartialMeasures::default()
    }
    .complete();
    assert_eq!(hung.bust, 88.0);
}

#[test]
fn the_lower_body_triangle_closes() {
    let from_outseam = PartialMeasures {
        outseam: Some(104.0),
        inseam: Some(78.0),
        ..PartialMeasures::default()
    }
    .complete();
    assert_eq!(from_outseam.rise, 26.0);
    let from_inseam = PartialMeasures {
        inseam: Some(78.0),
        rise: Some(27.0),
        ..PartialMeasures::default()
    }
    .complete();
    assert_eq!(from_inseam.outseam, 105.0);
}
