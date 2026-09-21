use std::collections::BTreeSet;

use toile_seamly::{Construction, Cubic, Evaluation, Object, ObjectKind, SplineLength, Xy};

use super::{distance, evaluate, oracle, pattern};

/// A composite Simpson rule: a different method from the crate's, dense
/// enough that its own error is far below the tolerance it checks.
fn simpson(cubic: &Cubic, panels: u32) -> f64 {
    let speed = |t: f64| {
        let [x, y] = cubic.velocity(t);
        x.hypot(y)
    };
    let h = 1.0 / f64::from(panels);
    let mut sum = speed(0.0) + speed(1.0);
    for k in 1..panels {
        let weight = if k % 2 == 1 { 4.0 } else { 2.0 };
        sum += weight * speed(f64::from(k) * h);
    }
    sum * h / 3.0
}

fn curves(evaluation: &Evaluation) -> impl Iterator<Item = &Cubic> {
    evaluation
        .splines
        .values()
        .chain(evaluation.paths.values().flatten())
}

/// A point compared to the bit.
fn bits(xy: Xy) -> [u64; 2] {
    xy.map(f64::to_bits)
}

fn name(object: Option<&Object>) -> &str {
    object.and_then(Object::name).expect("the id names a point")
}

#[test]
fn adaptive_quadrature_agrees_with_a_dense_simpson_rule_to_1e_10_cm() {
    let evaluation = evaluate(SplineLength::ArcLength);
    let (mut worst_gap, mut worst_estimate, mut count) = (0.0_f64, 0.0_f64, 0);
    for cubic in curves(&evaluation) {
        let quadrature = cubic.arc_length(1.0);
        let gap = (quadrature.value - simpson(cubic, 1 << 16)).abs();
        assert!(gap <= 1e-10, "{cubic:?}: {gap:e} cm from the dense rule");
        worst_gap = worst_gap.max(gap);
        worst_estimate = worst_estimate.max(quadrature.error);
        count += 1;
    }
    assert!(count > 0);
    println!(
        "arc length: {count} curves, worst gap to Simpson {worst_gap:e} cm, worst error estimate {worst_estimate:e} cm"
    );
}

#[test]
fn a_spline_with_both_handles_at_zero_measures_exactly_its_chord() {
    let pattern = pattern();
    let evaluation = evaluate(SplineLength::ArcLength);
    let mut straight = 0;
    for object in pattern.objects() {
        let ObjectKind::Spline(spline) = &object.kind else {
            continue;
        };
        let handles = [&spline.length1, &spline.length2].map(|f| evaluation.eval(f).unwrap());
        if handles == [0.0, 0.0] {
            let cubic = evaluation.splines[&object.id];
            let gap = (cubic.length() - distance(cubic.0[0], cubic.0[3])).abs();
            assert!(
                gap <= 1e-12,
                "spline {}: {gap:e} cm off its chord",
                object.id
            );
            straight += 1;
        }
    }
    assert!(straight > 0);
}

/// Checked with the file's written lengths, so that every end the owner's
/// evaluator places can be compared with it.
#[test]
fn every_spline_leaves_and_reaches_its_points_along_its_handles() {
    let pattern = pattern();
    let evaluation = evaluate(SplineLength::Written);
    let oracle = oracle();
    for object in pattern.objects() {
        let ObjectKind::Spline(spline) = &object.kind else {
            continue;
        };
        let [p0, p1, p2, p3] = evaluation.splines[&object.id].0;
        for (end, id) in [(p0, spline.start), (p3, spline.end)] {
            assert_eq!(bits(end), bits(evaluation.points[&id].at));
            // The oracle places everything but the cut points.
            if let Some(want) = oracle.points.get(name(pattern.object(id))) {
                assert!(distance(end, *want) <= 1e-9);
            }
        }
        let handles = [
            (p0, p1, &spline.length1, &spline.angle1),
            (p3, p2, &spline.length2, &spline.angle2),
        ];
        for (from, to, length, angle) in handles {
            let (length, angle) = (
                evaluation.eval(length).unwrap(),
                evaluation.eval(angle).unwrap(),
            );
            assert!((distance(from, to) - length).abs() <= 1e-12);
            if length > 0.0 {
                let heading = (from[1] - to[1]).atan2(to[0] - from[0]).to_degrees();
                let turn = (heading - angle).rem_euclid(360.0);
                assert!(
                    turn.min(360.0 - turn) <= 1e-9,
                    "spline {}: handle off by {turn}°",
                    object.id
                );
            }
        }
    }
}

#[test]
fn a_spline_path_runs_through_its_points_in_order() {
    let pattern = pattern();
    let evaluation = evaluate(SplineLength::ArcLength);
    let oracle = oracle();
    for object in pattern.objects() {
        let ObjectKind::SplinePath(points) = &object.kind else {
            continue;
        };
        let segments = &evaluation.paths[&object.id];
        assert_eq!(segments.len() + 1, points.len());
        for (segment, pair) in segments.iter().zip(points.windows(2)) {
            assert!(
                distance(
                    segment.0[0],
                    oracle.points[name(pattern.object(pair[0].point))]
                ) <= 1e-9
            );
            assert!(
                distance(
                    segment.0[3],
                    oracle.points[name(pattern.object(pair[1].point))]
                ) <= 1e-9
            );
        }
    }
}

/// With the written lengths every arc end lands on a point the owner's
/// evaluator placed; with integrated ones, on a point this crate placed.
#[test]
fn every_arc_starts_and_ends_on_its_circle_at_a_placed_point() {
    let oracle = oracle();
    for lengths in [SplineLength::Written, SplineLength::ArcLength] {
        let evaluation = evaluate(lengths);
        let placed: Vec<Xy> = match lengths {
            SplineLength::Written => oracle.points.values().copied().collect(),
            SplineLength::ArcLength => evaluation.points.values().map(|p| p.at).collect(),
        };
        assert!(!evaluation.arcs.is_empty());
        for (id, arc) in &evaluation.arcs {
            for end in [arc.start, arc.end] {
                assert!((distance(arc.center, end) - arc.radius).abs() <= 1e-12);
                let nearest = placed
                    .iter()
                    .map(|p| distance(*p, end))
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    nearest <= 1e-9,
                    "arc {id}: an end {nearest:e} cm from any point"
                );
            }
            assert!(arc.sweep() > 0.0);
        }
    }
}

/// The owner's notes on the fly: the centre front runs straight 13.19 cm
/// from the waist to the hip, then curves 10.72 cm to the crotch, and the
/// 16 cm opening ends 2.81 cm into the curve, 0.18 cm off the straight line;
/// 17 and 19 cm down it is 0.34 and 0.92 cm off.
#[test]
fn the_fly_opening_lands_where_the_owners_notes_measured_it() {
    let evaluation = evaluate(SplineLength::ArcLength);
    let point = |name: &str| evaluation.point_named(name).expect("the pattern names it");
    let (waist, hip) = (point("waist_margin_end"), point("hip_end"));
    let straight = evaluation.drawn("Spl_waist_margin_end_hip_end").unwrap();
    assert!((straight - distance(waist, hip)).abs() <= 1e-12);
    assert!((straight - 13.19).abs() < 0.005);
    // Its written length is still current: the file's four decimals agree.
    assert!((straight - 13.1856).abs() < 5e-5);

    let (hook, cut) = evaluation
        .cuts
        .iter()
        .map(|(id, cut)| (evaluation.points[id].at, *cut))
        .next()
        .expect("the fly hook is a cut");
    assert_eq!(bits(hook), bits(point("ff_hook")));
    let curve = evaluation.splines[&cut.spline];
    assert!((curve.length() - 10.72).abs() < 0.005);
    assert!((straight + cut.length - 16.0).abs() <= 1e-12);
    assert!((curve.arc_length(cut.t).value - cut.length).abs() <= 1e-11);
    assert_eq!(bits(curve.point(cut.t)), bits(hook));

    let (dx, dy) = (hip[0] - waist[0], hip[1] - waist[1]);
    let off_line = |p: Xy| ((p[0] - waist[0]) * dy - (p[1] - waist[1]) * dx).abs() / dx.hypot(dy);
    // The notes give hundredths, and each row lands within one of them.
    for (down, measured) in [(16.0, 0.18), (17.0, 0.34), (19.0, 0.92)] {
        let t = curve
            .t_at_length(down - straight)
            .expect("inside the curve");
        let off = off_line(curve.point(t));
        assert!(
            (off - measured).abs() < 0.01,
            "{down} cm down: {off} cm off, noted {measured}"
        );
        println!(
            "fly: {down} cm down, {:.4} cm into the curve at t = {t:.9}, {off:.4} cm off the line",
            down - straight
        );
    }
}

/// Every `Spl_` a formula cites has a value; the report says which are
/// geometrically straight.
#[test]
fn every_spline_length_a_formula_cites_is_defined() {
    let pattern = pattern();
    let evaluation = evaluate(SplineLength::ArcLength);
    let mut cited = BTreeSet::new();
    for object in pattern.objects() {
        if let ObjectKind::Point {
            construction:
                Construction::AlongLine { length, .. }
                | Construction::EndLine { length, .. }
                | Construction::CutSpline { length, .. },
            ..
        } = &object.kind
        {
            cited.extend(
                length
                    .names()
                    .into_iter()
                    .filter(|n| n.starts_with("Spl_"))
                    .map(str::to_owned),
            );
        }
    }
    assert!(!cited.is_empty());
    for spline_name in &cited {
        let value = evaluation.drawn(spline_name).expect("cited, so defined");
        let cubic = pattern
            .objects()
            .find_map(|object| match &object.kind {
                ObjectKind::Spline(s)
                    if *spline_name
                        == format!(
                            "Spl_{}_{}",
                            name(pattern.object(s.start)),
                            name(pattern.object(s.end))
                        ) =>
                {
                    Some(evaluation.splines[&object.id])
                }
                _ => None,
            })
            .expect("a spline carries the name");
        let bow = value - distance(cubic.0[0], cubic.0[3]);
        let shape = if bow <= 1e-9 { "straight" } else { "curved" };
        println!("{spline_name} = {value:.9} cm, {bow:.9} cm longer than its chord: {shape}");
    }
}
