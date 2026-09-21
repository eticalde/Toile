#![allow(missing_docs, reason = "a test crate publishes no API surface")]

mod curves;
mod parity;
mod structure;

use std::collections::BTreeMap;

use serde_json::Value;
use toile_seamly::{Evaluation, Measurements, Pattern, SplineLength, Xy};

// The owner's pattern and body, copied byte for byte but for the body's
// personal block, whose details are replaced by sentinels; no evaluator reads
// that block. The oracle is what the owner's evaluator prints for them:
//   python3 ~/seamly2d/tools/eval_pattern.py <pattern.sm2d> <oracle.json>
const PATTERN: &str = include_str!("../fixtures/baggy-jeans.sm2d");
const BODY: &str = include_str!("../fixtures/measurements-eti.smis");
const ORACLE: &str = include_str!("../fixtures/baggy-jeans.oracle.json");

fn pattern() -> Pattern {
    Pattern::parse(PATTERN).expect("the owner's pattern reads")
}

fn body() -> Measurements {
    Measurements::parse(BODY).expect("the owner's measurements read")
}

fn evaluate(lengths: SplineLength) -> Evaluation {
    Evaluation::new(&pattern(), &body(), lengths).expect("the owner's pattern evaluates")
}

/// The owner's evaluator's answer: points by name, and the value of every
/// measurement and variable.
struct Oracle {
    points: BTreeMap<String, Xy>,
    values: BTreeMap<String, f64>,
}

fn oracle() -> Oracle {
    let json: Value = serde_json::from_str(ORACLE).expect("the oracle is JSON");
    let number = |v: &Value| v.as_f64().expect("the oracle writes numbers");
    let points = json["P"]
        .as_object()
        .expect("the oracle has points")
        .iter()
        .map(|(name, xy)| (name.clone(), [number(&xy[0]), number(&xy[1])]))
        .collect();
    let values = json["V"]
        .as_object()
        .expect("the oracle has values")
        .iter()
        .map(|(name, v)| (name.clone(), number(v)))
        .collect();
    Oracle { points, values }
}

/// Every evaluated point by name; the names are unique, as formulas need.
fn by_name(evaluation: &Evaluation) -> BTreeMap<String, Xy> {
    let mut named = BTreeMap::new();
    for point in evaluation.points.values() {
        let earlier = named.insert(point.name.clone(), point.at);
        assert!(earlier.is_none(), "two points are called `{}`", point.name);
    }
    named
}

fn distance(a: Xy, b: Xy) -> f64 {
    (b[0] - a[0]).hypot(b[1] - a[1])
}
