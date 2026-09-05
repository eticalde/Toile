#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use std::collections::BTreeMap;

use toile_doc::formula::{Dependency, evaluation_order};
use toile_doc::{Binding, Command, Doc, PieceKey, PointKey, Segment, Winding, block};

/// The measurements of the chosen body, then the pattern variables in the
/// order each one's own reading demands.
fn environment(doc: &Doc) -> BTreeMap<String, f64> {
    let set = doc.measures().expect("the block chooses a body");
    let mut env = set.values.clone();
    let variables: Vec<(&str, &Binding)> = doc
        .variables
        .iter()
        .map(|(_, variable)| (variable.name.as_str(), &variable.value))
        .collect();
    let graph: Vec<Dependency<'_>> = variables
        .iter()
        .map(|&(name, binding)| Dependency {
            name,
            reads: binding.names(),
        })
        .collect();
    for index in evaluation_order(&graph).expect("the block has no cycle") {
        let (name, binding) = variables[index];
        let value = binding.eval(&env).expect("every name is bound by now");
        env.insert(name.to_owned(), value);
    }
    env
}

/// The piece's nodes resolved to centimetres, in contour order.
fn outline(doc: &Doc, piece: PieceKey) -> Vec<[f64; 2]> {
    let env = environment(doc);
    let held = doc.pieces.get(piece).expect("the key is live");
    held.anchors()
        .map(|key| {
            let point = doc.points.get(key).expect("the contour cites live points");
            [
                point.x.eval(&env).expect("the block resolves"),
                point.y.eval(&env).expect("the block resolves"),
            ]
        })
        .collect()
}

/// The straight run from node to node, which is all a document without a
/// flattening can measure. The hip and the crotch are curves, so the seam
/// itself is longer than its chords, and the engine is where that shows.
fn run_length(outline: &[[f64; 2]], from: usize, to: usize) -> f64 {
    outline[from..=to]
        .windows(2)
        .map(|pair| {
            let dx = pair[1][0] - pair[0][0];
            let dy = pair[1][1] - pair[0][1];
            dx.hypot(dy)
        })
        .sum()
}

fn front(doc: &Doc) -> PieceKey {
    doc.piece_named(block::FRONT).expect("the block draws one")
}

#[test]
fn etienne_resolves_the_nine_nodes_where_the_draft_says() {
    let doc = block::trouser_front();
    let expected = [
        [0.0, 0.0],
        [22.0, 0.0],
        [25.5, 20.0],
        [21.6875, 65.5],
        [20.6875, 104.0],
        [-1.3125, 104.0],
        [-2.3125, 65.5],
        [-6.125, 27.0],
        [0.0, 20.875],
    ];
    for (got, want) in outline(&doc, front(&doc)).iter().zip(expected) {
        assert!(
            (got[0] - want[0]).abs() < 1.0e-9,
            "{got:?} against {want:?}"
        );
        assert!(
            (got[1] - want[1]).abs() < 1.0e-9,
            "{got:?} against {want:?}"
        );
    }
}

#[test]
fn the_chords_of_the_side_seam_run_104_5_cm() {
    let doc = block::trouser_front();
    let side = run_length(&outline(&doc, front(&doc)), 1, 4);
    assert!((side - 104.476).abs() < 0.001, "the side seam reads {side}");
}

#[test]
fn etienne_resolves_the_inseam_to_77_2_cm() {
    let doc = block::trouser_front();
    let outline = outline(&doc, front(&doc));
    let inseam = run_length(&outline, 5, 7);
    assert!((inseam - 77.201).abs() < 0.001, "the inseam reads {inseam}");
}

#[test]
fn the_chords_of_the_front_run_two_and_a_half_metres_around() {
    let doc = block::trouser_front();
    let mut outline = outline(&doc, front(&doc));
    outline.push(outline[0]);
    let perimeter = run_length(&outline, 0, 9);
    assert!((perimeter - 255.215).abs() < 0.001, "it reads {perimeter}");
}

#[test]
fn changing_the_mannequin_keeps_the_node_count() {
    let mut doc = block::trouser_front();
    let piece = front(&doc);
    let etienne = outline(&doc, piece);
    Command::ResolveWith {
        mannequin: doc.mannequin_named("Talla 42").expect("it is there"),
    }
    .apply(&mut doc)
    .expect("the key is live");
    let size_42 = outline(&doc, piece);

    assert_eq!(etienne.len(), size_42.len());
    assert_eq!(size_42.len(), 9);
    assert_ne!(etienne, size_42);
    assert!((size_42[1][0] - 23.0).abs() < 1.0e-9);
    assert!((size_42[4][1] - 106.0).abs() < 1.0e-9);
}

#[test]
fn the_declared_winding_is_the_one_the_resolved_contour_runs_in() {
    let doc = block::trouser_front();
    let piece = front(&doc);
    let outline = outline(&doc, piece);
    let mut twice_area = 0.0;
    for i in 0..outline.len() {
        let a = outline[i];
        let b = outline[(i + 1) % outline.len()];
        twice_area += a[0] * b[1] - b[0] * a[1];
    }
    let declared = doc.pieces.get(piece).expect("the key is live").winding;
    assert_eq!(declared, Winding::of_area(twice_area));
    assert_eq!(declared, Winding::Cw);
}

#[test]
fn every_name_the_block_reads_is_a_name_the_document_binds() {
    let doc = block::trouser_front();
    let env = environment(&doc);
    for (_, point) in doc.points.iter() {
        for name in point.x.names().into_iter().chain(point.y.names()) {
            assert!(env.contains_key(name), "{name} is bound by nothing");
        }
    }
}

#[test]
fn a_variable_that_reads_a_measure_the_body_lacks_stops_the_resolution() {
    let mut doc = block::trouser_front();
    let key = doc.variable_named("extension_tiro").expect("it is there");
    Command::SetVariable {
        variable: key,
        to: Binding::parse("largo_manga / 2").expect("the source parses"),
    }
    .apply(&mut doc)
    .expect("the key is live");
    let env = doc
        .measures()
        .expect("the block chooses a body")
        .values
        .clone();
    let binding = &doc.variables.get(key).expect("the key is live").value;
    assert!(binding.eval(&env).is_err());
}

#[test]
fn the_hip_and_the_crotch_are_the_two_tracts_the_block_bends() {
    let doc = block::trouser_front();
    let held = doc.pieces.get(front(&doc)).expect("the key is live");
    let bent: Vec<(usize, u16)> = held
        .contour
        .iter()
        .enumerate()
        .filter(|(_, node)| node.segment != Segment::Line)
        .map(|(rank, node)| (rank, node.samples))
        .collect();
    assert_eq!(bent, [(1, 24), (7, 16)]);
    // A straight tract is one point of the flattening and nothing more, so a
    // sample count above one only ever belongs to a curve.
    for (rank, node) in held.contour.iter().enumerate() {
        let straight = node.segment == Segment::Line;
        assert_eq!(straight, node.samples == 1, "node {rank}");
    }
}

#[test]
fn a_handle_is_a_named_point_that_stands_on_no_node() {
    let doc = block::trouser_front();
    let piece = front(&doc);
    let held = doc.pieces.get(piece).expect("the key is live");
    let handles: Vec<PointKey> = held
        .contour
        .iter()
        .filter_map(|node| node.segment.handles())
        .flat_map(|(out, into)| [out, into])
        .collect();
    assert_eq!(handles.len(), 4);
    assert_eq!(doc.points.len(), 9 + handles.len());
    for handle in handles {
        let point = doc.points.get(handle).expect("the key is live");
        assert!(point.label.is_some(), "a handle carries a name of its own");
        assert_eq!(doc.label_of(piece, handle), None, "but no node shows it");
    }
}
