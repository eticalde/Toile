use std::collections::BTreeMap;

use toile_doc::Command;
use toile_seamly::{Frozen, Measurements, seamly_measurement};

use super::{body, distance, product, reference, resolve};

/// How close a product point must land on the pattern's, in centimetres.
const PARITY: f64 = 1e-6;

/// The change of body the parametric check makes: Toile's names, and how
/// many centimetres each grows.
const GROWTH: [(&str, f64); 2] = [("cadera", 4.0), ("entrepierna", 3.0)];

#[test]
fn every_point_of_the_product_lands_where_the_pattern_puts_it() {
    let product = product();
    let reference = reference(&body());
    let resolved = resolve(&product.doc);
    assert_eq!(resolved.len(), product.doc.points.len());
    let mut worst: f64 = 0.0;
    for (key, at) in &resolved {
        let source = product
            .sources
            .get(key)
            .expect("every point knows where it comes from");
        let want = source
            .locate(&reference)
            .expect("every source is evaluated");
        let gap = distance(*at, want);
        assert!(gap <= PARITY, "{source:?} lands {gap:e} cm off");
        worst = worst.max(gap);
    }
    println!("parity: {} points, worst {worst:e} cm", resolved.len());
}

/// The same body grown in the hip and the inseam, on both sides: through the
/// pattern's own evaluator, and through the product's formulas after the
/// same change to its body.
fn grown(body: &Measurements) -> (Measurements, Vec<Command>) {
    let product = product();
    let mannequin = product.doc.resolve_with;
    let mut seamly = body.clone();
    let mut commands = Vec::new();
    for (toile, by) in GROWTH {
        let name = seamly_measurement(toile).expect("the mapping names both");
        let value = body.get(name).expect("the owner's body carries both") + by;
        seamly = seamly.with_value(name, value).expect("the body has it");
        commands.push(Command::SetMeasure {
            mannequin,
            name: toile.to_owned(),
            to: value,
        });
    }
    (seamly, commands)
}

#[test]
fn a_grown_body_moves_the_product_as_it_moves_the_pattern_but_for_what_was_frozen() {
    let mut product = product();
    let (seamly, commands) = grown(&body());
    for command in commands {
        command
            .apply(&mut product.doc)
            .expect("the body carries the measure");
    }
    let reference = reference(&seamly);
    let resolved = resolve(&product.doc);
    let mut drift: BTreeMap<Frozen, f64> = BTreeMap::new();
    let mut free = 0;
    for (key, at) in &resolved {
        let want = product.sources[key].locate(&reference).expect("evaluated");
        let gap = distance(*at, want);
        match product.frozen.get(key) {
            None => {
                assert!(
                    gap <= PARITY,
                    "{:?} lands {gap:e} cm off",
                    product.sources[key]
                );
                free += 1;
            }
            Some(frozen) => {
                for item in frozen {
                    let worst = drift.entry(*item).or_default();
                    *worst = worst.max(gap);
                }
            }
        }
    }
    println!("grown: {free} points follow exactly; frozen drift {drift:?}");
    assert!(free > 0);
    for (item, worst) in &drift {
        assert!(*worst < 0.05, "{item:?} drifts {worst} cm");
    }
}

#[test]
fn only_the_back_yoke_depends_on_something_frozen() {
    let product = product();
    let frozen: Vec<_> = product
        .report
        .frozen
        .iter()
        .filter(|f| !f.reaches.is_empty())
        .collect();
    assert_eq!(frozen.len(), 1, "{frozen:?}");
    assert_eq!(frozen[0].name, "Spl_crotch_waist_margin_up");
    assert!(
        (frozen[0].value - 0.043_867).abs() < 1e-5,
        "{}",
        frozen[0].value
    );
    for label in [
        "bk_waist_side",
        "bk_waist_cb",
        "bk_dart_a",
        "bk_dart_tip",
        "bk_dart_b",
    ] {
        assert!(frozen[0].reaches.iter().any(|r| r == label), "{label}");
    }
    let hook = product
        .report
        .frozen
        .iter()
        .find(|f| f.name == "ff_hook")
        .expect("reported");
    assert!(hook.reaches.is_empty());
    assert!((hook.value - 0.114_469_430).abs() < 1e-9);
}

/// Every spline the file writes a length on is reported beside the curve's
/// own length and the points that cite it.
#[test]
fn every_written_spline_length_is_reported_beside_the_curve_and_its_citers() {
    let product = product();
    let evaluation = reference(&body());
    let lengths = &product.report.lengths;
    assert_eq!(lengths.len(), 11);
    for note in lengths {
        assert_eq!(
            evaluation.drawn(&note.name),
            Some(note.arc),
            "{}",
            note.name
        );
    }
    let citers = |name: &str| {
        lengths
            .iter()
            .find(|n| n.name == name)
            .map(|n| n.cited_by.clone())
    };
    assert_eq!(
        citers("Spl_crotch_waist_margin_up"),
        Some(vec!["bk_waist_side".to_owned()])
    );
    assert_eq!(
        citers("Spl_waist_margin_end_hip_end"),
        Some(vec!["ff_hook".to_owned()])
    );
    assert_eq!(citers("Spl_ff_top_ff_hook"), Some(Vec::new()));
}
