#![allow(missing_docs, reason = "a test crate publishes no API surface")]

mod body;
mod inner;
mod parity;
mod persona;
mod shape;
mod synthetic;

use std::collections::BTreeMap;

use toile_doc::formula::{Dependency, evaluation_order};
use toile_doc::{Doc, PointKey};
use toile_seamly::{Evaluation, Measurements, Pattern, Product, SplineLength, Xy, import};

// The owner's pattern and body, the fixtures the reader's tests read.
const PATTERN: &str = include_str!("../fixtures/baggy-jeans.sm2d");
const BODY: &str = include_str!("../fixtures/measurements-eti.smis");

/// The name the command line gives the body: the measurement file's stem.
const NAME: &str = "measurements-eti";

fn pattern() -> Pattern {
    Pattern::parse(PATTERN).expect("the owner's pattern reads")
}

fn body() -> Measurements {
    Measurements::parse(BODY).expect("the owner's measurements read")
}

fn product() -> Product {
    import(&pattern(), &body(), NAME).expect("the owner's pattern imports")
}

fn reference(body: &Measurements) -> Evaluation {
    Evaluation::new(&pattern(), body, SplineLength::ArcLength).expect("the pattern evaluates")
}

/// Every point of the document where its formulas put it, resolved the way
/// Toile resolves a document: the body's measurements, then the variables in
/// the order they read each other.
///
/// The engine's own resolution is checked against the same reference by the
/// command line's tests; this one keeps the crate's tests free of the engine.
fn resolve(doc: &Doc) -> BTreeMap<PointKey, Xy> {
    let body = doc
        .measures()
        .expect("the product resolves against its body");
    let mut env: BTreeMap<String, f64> = body.values.clone();
    let variables: Vec<_> = doc.variables.iter().map(|(_, v)| v).collect();
    let graph: Vec<Dependency<'_>> = variables
        .iter()
        .map(|v| Dependency {
            name: &v.name,
            reads: v.value.names(),
        })
        .collect();
    for index in evaluation_order(&graph).expect("no variable reads itself") {
        let variable = variables[index];
        let value = variable
            .value
            .eval(&env)
            .unwrap_or_else(|e| panic!("{}: {e}", variable.name));
        env.insert(variable.name.clone(), value);
    }
    doc.points
        .iter()
        .map(|(key, point)| {
            let x = point
                .x
                .eval(&env)
                .unwrap_or_else(|e| panic!("{key:?} x: {e}"));
            let y = point
                .y
                .eval(&env)
                .unwrap_or_else(|e| panic!("{key:?} y: {e}"));
            (key, [x, y])
        })
        .collect()
}

fn distance(a: Xy, b: Xy) -> f64 {
    (b[0] - a[0]).hypot(b[1] - a[1])
}
