#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use toile_doc::formula::Formula;
use toile_doc::{Axis, Binding, Command, Doc, History, MeasureSet, PointKey, block};

fn env() -> BTreeMap<String, f64> {
    [("a", 3.0), ("b", 7.0), ("cintura", 84.0), ("cadera", 98.0)]
        .into_iter()
        .map(|(n, v)| (n.to_owned(), v))
        .collect()
}

fn nudged(src: &str, delta: f64, step: f64) -> String {
    Formula::parse(src)
        .expect("the source parses")
        .nudged_source(delta, step)
}

#[test]
fn probe_nudge_shapes() {
    let cases: &[(&str, f64, f64)] = &[
        ("cadera / 4", 0.6, 0.1),
        ("cadera / 4", -0.6, 0.1),
        ("22", 0.6, 0.1),
        ("22", -0.6, 0.1),
        ("-22", 0.6, 0.1),
        ("cintura / 4 + 1", 0.004, 0.0),
        ("cintura / 4 + 1", 0.4, 1.0),
        ("cintura / 4 + 1", -1.0, 0.1),
        ("cintura / 4 + 0", -1.0, 0.1),
        ("a < b ? 1 : a / 2", 0.6, 0.1),
        ("a < b ? (1 + 2) : 3", 0.6, 0.1),
        ("(a < b ? 1 : 2)", 0.6, 0.1),
        ("a < b ? 1 : a < b ? 2 : 3", 0.6, 0.1),
        ("a < b ? a < b ? 1 : 2 : 3", 0.6, 0.1),
        ("min(1, 2)", 0.5, 0.1),
        ("a ^ 2", 0.6, 0.1),
        ("a - b - 1", 0.6, 0.1),
        ("a - -5", 0.6, 0.1),
        ("cadera / 4  ", 0.6, 0.1),
        ("cintura / 4 + 1", 1e300, 0.1),
        ("1", 0.0049, 0.0),
        ("1", 0.005, 0.0),
        ("1", -0.005, 0.0),
        ("0.005", -0.005, 0.0),
        ("a + 0.004", 0.004, 0.0),
    ];
    let e = env();
    for &(src, delta, step) in cases {
        let out = nudged(src, delta, step);
        let parsed = Formula::parse(&out);
        let sum = match (
            Formula::parse(src).unwrap().eval(&e),
            parsed.as_ref().map(|f| f.eval(&e)),
        ) {
            (Ok(here), Ok(Ok(there))) => format!("{here} -> {there} (want +{delta})"),
            _ => "unresolved".to_owned(),
        };
        println!(
            "{src:?} +{delta} step {step} => {out:?} parses={} | {sum}",
            parsed.is_ok()
        );
        assert!(parsed.is_ok(), "{src:?} nudged wrote {out:?}");
    }
}

#[test]
fn probe_undo_across_a_measure_set_switch() {
    let mut doc = block::trouser_front();
    let other = doc
        .mannequins
        .insert(MeasureSet::new("Otro", [("cintura", 90.0)]));
    let first = doc.resolve_with;
    let before = doc.to_canonical_json();
    let waist = {
        let front = doc.piece_named(block::FRONT).expect("one piece");
        doc.shows_label(front, "cintura_lat").expect("named")
    };
    let mut history = History::new();

    history.begin("cambiar de cuerpo");
    history
        .edit(&mut doc, Command::ResolveWith { mannequin: other })
        .expect("the body is live");
    history.end();

    history.begin("mover punto");
    for x in [22.1_f64, 22.4, 23.5] {
        history
            .edit(
                &mut doc,
                Command::MovePoint {
                    point: waist,
                    to: [Binding::literal(x), Binding::literal(0.0)],
                },
            )
            .expect("the point is live");
    }
    history.end();
    let after = doc.to_canonical_json();

    history.undo(&mut doc).expect("the drag undoes");
    history.undo(&mut doc).expect("the switch undoes");
    assert_eq!(doc.resolve_with, first);
    assert_eq!(
        doc.to_canonical_json(),
        before,
        "undo did not restore bytes"
    );

    history.redo(&mut doc).expect("the switch redoes");
    history.redo(&mut doc).expect("the drag redoes");
    assert_eq!(doc.to_canonical_json(), after, "redo did not restore bytes");
}

#[test]
fn probe_coalesced_drag_bytes() {
    let mut doc = block::trouser_front();
    let waist = {
        let front = doc.piece_named(block::FRONT).expect("one piece");
        doc.shows_label(front, "cintura_lat").expect("named")
    };
    let before = doc.to_canonical_json();
    let mut history = History::new();
    history.begin("mover punto");
    // A drag over a formula: every frame nudges the ORIGIN, as the app does.
    let origin = doc.points.get(waist).expect("live").x.clone();
    for delta in [0.1_f64, 0.4, 0.9, 1.6] {
        let Binding::Formula(formula) = &origin else {
            panic!("the waist is a formula")
        };
        let to = Binding::Formula(formula.nudge(delta, 0.1).expect("it nudges"));
        history
            .edit(
                &mut doc,
                Command::SetBinding {
                    point: waist,
                    axis: Axis::X,
                    to,
                },
            )
            .expect("the point is live");
    }
    history.end();
    assert_eq!(history.depth(), 1);
    let after = doc.to_canonical_json();
    history.undo(&mut doc).expect("the drag undoes");
    assert_eq!(doc.to_canonical_json(), before);
    history.redo(&mut doc).expect("the drag redoes");
    assert_eq!(doc.to_canonical_json(), after);
    history.undo(&mut doc).expect("again");
    assert_eq!(doc.to_canonical_json(), before);
}

#[test]
fn probe_malformed_files() {
    let cases: &[&str] = &[
        "",
        " ",
        "null",
        "0",
        "\"toile\"",
        "{\"toile\": 1}",
        "{\"toile\": 1, \"doc\": null}",
        "{\"toile\": 1.0, \"doc\": {}}",
        "{\"toile\": -1, \"doc\": {}}",
        "{\"toile\": 18446744073709551615, \"doc\": {}}",
        "{\"toile\": 1, \"doc\": {}, \"toile\": 2}",
    ];
    for case in cases {
        let read = Doc::from_json(case);
        println!("{case:?} => {read:?}", read = read.as_ref().err());
        assert!(read.is_err(), "{case:?} was taken for a pattern");
    }
    let deep = format!(
        "{}{}",
        "{\"doc\":".repeat(2000),
        "1".to_owned() + &"}".repeat(2000)
    );
    assert!(Doc::from_json(&deep).is_err());
    let _ = PointKey::new(0, 0);
}

#[test]
fn probe_implausible_slot_counts() {
    // A store's slot count decides how much the reader allocates, so a file
    // naming more slots than a pattern can hold has to be refused before the
    // reader opens them, not after.
    for issued in ["1000001", "4000000000"] {
        let store = |name: &str| {
            let count = if name == "points" { issued } else { "0" };
            format!("\"{name}\":{{\"issued\":{count},\"entries\":[]}}")
        };
        let stores: Vec<String> = [
            "pieces",
            "points",
            "seams",
            "notches",
            "darts",
            "symmetries",
            "pins",
            "variables",
        ]
        .into_iter()
        .map(store)
        .collect();
        let mannequins = "\"mannequins\":{\"issued\":1,\"entries\":\
            [{\"id\":\"0.0\",\"name\":\"E\",\"values\":{}}]}";
        let text = format!(
            "{{\"toile\":1,\"doc\":{{{},{mannequins},\"resolve_with\":\"0.0\"}}}}",
            stores.join(",")
        );
        let started = Instant::now();
        let read = Doc::from_json(&text);
        let took = started.elapsed();
        println!(
            "issued {issued} => {read:?} in {took:?}",
            read = read.as_ref().err()
        );
        assert!(read.is_err(), "{issued} slots were taken for a pattern");
        assert!(
            took < Duration::from_secs(5),
            "{issued} slots took {took:?}"
        );
    }
}
