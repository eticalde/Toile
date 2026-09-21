use std::collections::BTreeMap;

use toile_engine::draft::{Command, Draft, PointKey};
use toile_seamly::{
    Evaluation, Frozen, Measurements, Pattern, Product, SplineLength, Xy, seamly_measurement,
};

/// How close a point of the product must land on the pattern's, in
/// centimetres, for the two to be the same point.
pub const PARITY: f64 = 1e-6;

/// The change of body the report measures the product against: catalogue
/// names, and how many centimetres each grows.
pub const GROWTH: [(&str, f64); 2] = [("cadera", 4.0), ("entrepierna", 3.0)];

/// What resolving the product in Toile shows, next to the pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    /// Points compared.
    pub points: usize,
    /// The farthest any of them lands from the pattern's, on the imported
    /// body, in centimetres.
    pub worst: f64,
    /// The measurements actually grown, with their new values.
    pub grown: Vec<(String, f64)>,
    /// On the grown body: points that depend on nothing frozen, and the
    /// farthest any of them lands from the pattern's.
    pub followed: (usize, f64),
    /// On the grown body: how far each frozen quantity moves the points it
    /// reaches, or would move its cut point, from where the pattern puts
    /// them, in centimetres.
    pub drift: BTreeMap<Frozen, f64>,
    /// Every defect a piece resolves with, in words.
    pub defects: Vec<String>,
    /// What an evaluator that trusts the spline lengths the file wrote does
    /// differently; `None` when such an evaluator cannot evaluate the file,
    /// because a spline writes no length or a written one leaves a
    /// construction without an answer.
    pub trusting: Option<Trusting>,
}

/// The points of the product that an evaluator reading the file's spline
/// lengths, instead of measuring the curves, places elsewhere than Toile.
#[derive(Debug, Clone, PartialEq)]
pub struct Trusting {
    /// Their labels, in key order.
    pub points: Vec<String>,
    /// The farthest any of them lands from Toile's on the imported body, in
    /// centimetres.
    pub worst: f64,
    /// The same on the grown body; `None` if that evaluator cannot evaluate
    /// the grown body at all.
    pub grown: Option<f64>,
}

/// Resolves the product in Toile's engine, on its own body and on the grown
/// one, and compares every point with the pattern's own evaluation.
///
/// # Errors
/// The engine's refusal to resolve the product or to grow its body, or the
/// pattern's refusal to evaluate for the grown body.
pub fn check(product: &Product, pattern: &Pattern, body: &Measurements) -> Result<Check, String> {
    let evaluate = |body: &Measurements| {
        Evaluation::new(pattern, body, SplineLength::ArcLength).map_err(|e| e.to_string())
    };
    let trusted = |body: &Measurements| Evaluation::new(pattern, body, SplineLength::Written).ok();
    let mut draft = Draft::from_doc(product.doc.clone()).map_err(|e| e.to_string())?;
    let reference = evaluate(body)?;
    let written = trusted(body);
    let mut worst: f64 = 0.0;
    let mut moved: BTreeMap<PointKey, f64> = BTreeMap::new();
    for (key, source) in &product.sources {
        let toile = draft.resolved(*key);
        worst = worst.max(gap(toile, source.locate(&reference)));
        if let Some(there) = written.as_ref().map(|w| source.locate(w))
            && there != source.locate(&reference)
        {
            moved.insert(*key, gap(toile, there));
        }
    }
    let defects = defects(&draft);
    let (seamly, grown) = grow(&mut draft, body)?;
    let wider = evaluate(&seamly)?;
    let trusting = written.map(|_| Trusting {
        points: moved
            .keys()
            .filter_map(|key| product.doc.points.get(*key)?.label.clone())
            .collect(),
        worst: moved.values().fold(0.0, |a, &b| a.max(b)),
        grown: trusted(&seamly).map(|w| {
            moved
                .keys()
                .map(|key| gap(draft.resolved(*key), product.sources[key].locate(&w)))
                .fold(0.0, f64::max)
        }),
    });
    let mut followed = (0, 0.0_f64);
    let mut drift: BTreeMap<Frozen, f64> = BTreeMap::new();
    for (key, source) in &product.sources {
        let off = gap(draft.resolved(*key), source.locate(&wider));
        match product.frozen.get(key) {
            None => followed = (followed.0 + 1, followed.1.max(off)),
            Some(frozen) => {
                for item in frozen {
                    let held = drift.entry(*item).or_default();
                    *held = held.max(off);
                }
            }
        }
    }
    for note in &product.report.frozen {
        if let (true, Frozen::CutParameter(id)) = (note.reaches.is_empty(), note.frozen) {
            drift.insert(note.frozen, cut_drift(&wider, id, note.value));
        }
    }
    Ok(Check {
        points: product.sources.len(),
        worst,
        grown,
        followed,
        drift,
        defects,
        trusting,
    })
}

/// Every defect a piece of the draft resolves with, in words.
fn defects(draft: &Draft) -> Vec<String> {
    draft
        .doc()
        .pieces
        .iter()
        .flat_map(|(key, piece)| {
            let name = piece.name.clone();
            draft
                .defects(key)
                .iter()
                .map(move |d| format!("{name}: {d}"))
        })
        .collect()
}

/// Grows the draft's body and the pattern's alike, by `GROWTH`; the grown
/// Seamly body and the measurements actually grown, with their new values.
fn grow(
    draft: &mut Draft,
    body: &Measurements,
) -> Result<(Measurements, Vec<(String, f64)>), String> {
    let mut seamly = body.clone();
    let mut grown = Vec::new();
    for (toile, by) in GROWTH {
        let (Some(name), Some(set)) = (seamly_measurement(toile), draft.doc().measures()) else {
            continue;
        };
        let (Some(was), true) = (body.get(name), set.has(toile)) else {
            continue;
        };
        seamly = seamly
            .with_value(name, was + by)
            .ok_or("the body lost a measurement")?;
        let mannequin = draft.doc().resolve_with;
        let command = Command::SetMeasure {
            mannequin,
            name: toile.to_owned(),
            to: was + by,
        };
        draft.edit(command).map_err(|e| e.to_string())?;
        grown.push((toile.to_owned(), was + by));
    }
    Ok((seamly, grown))
}

/// How far from where the pattern cuts it a cut point lands when its curve
/// parameter is kept at `t`.
fn cut_drift(evaluation: &Evaluation, id: u32, t: f64) -> f64 {
    let (Some(cut), Some(point)) = (evaluation.cuts.get(&id), evaluation.points.get(&id)) else {
        return f64::NAN;
    };
    let frozen = evaluation
        .splines
        .get(&cut.spline)
        .map(|cubic| cubic.point(t));
    gap(frozen, Some(point.at))
}

/// The distance between two places, infinite when either is missing.
fn gap(a: Option<Xy>, b: Option<Xy>) -> f64 {
    match (a, b) {
        (Some(a), Some(b)) => (b[0] - a[0]).hypot(b[1] - a[1]),
        _ => f64::INFINITY,
    }
}
