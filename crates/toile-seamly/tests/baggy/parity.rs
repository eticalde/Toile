use toile_seamly::{Construction, ObjectKind, SplineLength};

use super::{body, by_name, distance, evaluate, oracle, pattern};

/// The owner's evaluator and this crate agree on every point it places, fed
/// the same spline lengths: the ones the file wrote, which it trusts.
#[test]
fn every_point_the_owner_evaluator_places_matches_within_1e_9_cm() {
    let ours = evaluate(SplineLength::Written);
    let named = by_name(&ours);
    let oracle = oracle();
    assert!(!oracle.points.is_empty());
    let mut worst: f64 = 0.0;
    for (name, want) in &oracle.points {
        let got = named
            .get(name)
            .unwrap_or_else(|| panic!("the oracle's `{name}` was not evaluated"));
        let gap = distance(*got, *want);
        assert!(gap <= 1e-9, "`{name}` lands {gap:e} cm from the oracle");
        worst = worst.max(gap);
    }
    // All the oracle leaves out is what it cannot evaluate: the cut points.
    for (id, point) in &ours.points {
        if !oracle.points.contains_key(&point.name) {
            assert!(
                ours.cuts.contains_key(id),
                "`{}` is missing from the oracle",
                point.name
            );
        }
    }
    println!(
        "parity: {} of {} points compared, worst {worst:e} cm",
        oracle.points.len(),
        ours.points.len()
    );
}

#[test]
fn every_measurement_and_variable_the_owner_evaluator_reports_matches_to_the_bit() {
    let ours = evaluate(SplineLength::ArcLength);
    let body = body();
    let oracle = oracle();
    assert!(!oracle.values.is_empty());
    for (name, want) in &oracle.values {
        let got = if name.starts_with('#') {
            ours.variable(name)
        } else {
            body.get(name)
        };
        let got = got.unwrap_or_else(|| panic!("`{name}` was not evaluated"));
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "`{name}`: {got} against {want}"
        );
    }
    assert_eq!(
        oracle.values.len(),
        body.entries().len() + pattern().variables.len()
    );
}

/// With lengths integrated, a point moves only if a stale written length
/// reaches it; everything else still matches the oracle, and the one point
/// that cites a spline length moves by exactly the staleness.
#[test]
fn integrated_spline_lengths_move_only_what_a_stale_written_length_reaches() {
    let pattern = pattern();
    let reference = evaluate(SplineLength::ArcLength);
    let written = evaluate(SplineLength::Written);
    let (reference_named, written_named) = (by_name(&reference), by_name(&written));
    let oracle = oracle();

    let (mut moved, mut worst_kept, mut worst_moved) = (Vec::new(), 0.0_f64, 0.0_f64);
    for (name, want) in &oracle.points {
        let (ours, theirs) = (reference_named[name], written_named[name]);
        let gap = distance(ours, *want);
        if distance(ours, theirs) == 0.0 {
            assert!(gap <= 1e-9, "`{name}` lands {gap:e} cm from the oracle");
            worst_kept = worst_kept.max(gap);
        } else {
            moved.push(name.clone());
            worst_moved = worst_moved.max(gap);
        }
    }

    let citers: Vec<(String, String)> = pattern
        .objects()
        .filter_map(|object| {
            let ObjectKind::Point {
                name,
                construction:
                    Construction::AlongLine { length, .. } | Construction::EndLine { length, .. },
                ..
            } = &object.kind
            else {
                return None;
            };
            let spline = length.names().into_iter().find(|n| n.starts_with("Spl_"))?;
            Some((name.clone(), spline.to_owned()))
        })
        .collect();
    assert!(!citers.is_empty());
    for (name, spline) in &citers {
        let staleness = (reference.drawn(spline).unwrap() - written.drawn(spline).unwrap()).abs();
        let shift = distance(reference_named[name], written_named[name]);
        assert!(
            (shift - staleness).abs() <= 1e-12,
            "`{name}` moved {shift:e} cm for a {staleness:e} cm change of `{spline}`"
        );
        assert!(moved.contains(name));
        println!(
            "stale: `{spline}` written {} against {} integrated",
            written.drawn(spline).unwrap(),
            reference.drawn(spline).unwrap()
        );
    }
    assert!(
        worst_moved < 1e-3,
        "the staleness grew to {worst_moved:e} cm"
    );
    println!(
        "reference: {} points within {worst_kept:e} cm, {} moved by the stale length, worst {worst_moved:e} cm",
        oracle.points.len() - moved.len(),
        moved.len()
    );
}
