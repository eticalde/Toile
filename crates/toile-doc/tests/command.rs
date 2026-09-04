#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_doc::formula::Formula;
use toile_doc::{
    Applied, Axis, Binding, ChangeClass, Command, ContourNode, Dart, DartKey, DartWedge, Doc,
    DocError, EdgeAnchor, EdgeRange, FoldDirection, Grain, Identity, MeasureSet, Notch, NotchKey,
    Piece, PieceKey, Pin, PinKey, Point, PointKey, Seam, SeamKey, SeamOrientation, Segment,
    Symmetry, SymmetryKey, SymmetryKind, WedgeNode, Winding, block,
};

fn front(doc: &Doc) -> PieceKey {
    doc.piece_named(block::FRONT).expect("the block draws one")
}

fn node(doc: &Doc, label: &str) -> PointKey {
    doc.shows_label(front(doc), label)
        .unwrap_or_else(|| panic!("the block names {label}"))
}

fn formula(source: &str) -> Binding {
    Binding::Formula(Formula::parse(source).expect("the source parses"))
}

/// The edits this phase implements, each one changing something.
fn implemented(doc: &Doc) -> Vec<Command> {
    let piece = front(doc);
    let waist = node(doc, "cintura_lat");
    vec![
        Command::MovePoint {
            point: waist,
            to: [Binding::literal(30.0), Binding::literal(2.0)],
        },
        Command::SetBinding {
            point: waist,
            axis: Axis::X,
            to: formula("cintura / 4 + 2"),
        },
        Command::SetVariable {
            variable: doc.variable_named("holgura_cadera").expect("it is there"),
            to: formula("cadera / 98"),
        },
        Command::SetMeasure {
            mannequin: doc.resolve_with,
            name: "cadera".to_owned(),
            to: 102.0,
        },
        Command::ResolveWith {
            mannequin: doc.mannequin_named("Talla 42").expect("it is there"),
        },
        Command::RenamePiece {
            piece,
            to: "Delantero derecho".to_owned(),
        },
        Command::SetGrain {
            piece,
            to: Grain::Angle(0.0),
        },
        Command::LabelPoint {
            point: waist,
            to: Some("cintura_costado".to_owned()),
        },
        Command::LabelPoint {
            point: waist,
            to: None,
        },
        Command::ShowLabel {
            point: waist,
            to: true,
        },
    ]
}

#[test]
fn apply_then_invert_is_identity() {
    let original = block::trouser_front();
    for command in implemented(&original) {
        let mut doc = original.clone();
        let Applied { inverse, .. } = command
            .clone()
            .apply(&mut doc)
            .unwrap_or_else(|error| panic!("{command:?} applies: {error}"));
        assert_ne!(doc, original, "{command:?} changed nothing");
        inverse
            .clone()
            .apply(&mut doc)
            .unwrap_or_else(|error| panic!("{inverse:?} applies: {error}"));
        assert_eq!(doc, original, "{inverse:?} did not undo {command:?}");
    }
}

#[test]
fn undoing_an_edit_gives_back_the_formula_and_not_its_value() {
    let mut doc = block::trouser_front();
    let waist = node(&doc, "cintura_lat");
    let before = doc.points.get(waist).expect("the key is live").x.clone();
    let applied = Command::MovePoint {
        point: waist,
        to: [Binding::literal(23.6), Binding::literal(0.0)],
    }
    .apply(&mut doc)
    .expect("the key is live");
    applied.inverse.apply(&mut doc).expect("the key is live");
    let after = &doc.points.get(waist).expect("the key is live").x;
    assert_eq!(after, &before);
    assert_eq!(after.source(), "cintura / 4 + 1");
}

/// Every command that changes the topology of a piece.
fn topology(doc: &Doc) -> Vec<Command> {
    let piece = front(doc);
    let point = node(doc, "cintura_cf");
    let anchor = EdgeAnchor::at_node(piece, point);
    let range = EdgeRange::between(piece, point, point);
    let leg = WedgeNode::line(Identity::New, Point::at(0.0, 0.0));
    vec![
        Command::InsertNode {
            piece,
            after: Some(point),
            identity: Identity::New,
            node: ContourNode::line(point),
            value: Point::at(0.0, 0.0),
        },
        Command::RemoveNode { piece, node: point },
        Command::SetSegment {
            piece,
            node: point,
            to: Segment::Line,
        },
        Command::SetSamples {
            piece,
            node: point,
            to: 8,
        },
        Command::AddPiece {
            identity: Identity::New,
            piece: Piece::polygon("Trasero", [point], Winding::Cw),
        },
        Command::RemovePiece { piece },
        Command::AddSeam {
            identity: Identity::New,
            seam: Seam::plain(range, range, SeamOrientation::Opposed),
        },
        Command::RemoveSeam {
            seam: SeamKey::new(0, 0),
        },
        Command::AddNotch {
            identity: Identity::New,
            notch: Notch::lone(anchor),
            mate: Some((Identity::New, Notch::lone(anchor))),
        },
        Command::MoveNotch {
            notch: NotchKey::new(0, 0),
            to: anchor,
        },
        Command::RemoveNotch {
            notch: NotchKey::new(0, 0),
        },
        Command::AddDart {
            identity: Identity::New,
            dart: Dart {
                apex: point,
                legs: (point, point),
                seam: SeamKey::new(0, 0),
                fold: FoldDirection::TowardEnd,
            },
            wedge: Box::new(DartWedge {
                piece,
                after: Some(point),
                nodes: [leg.clone(), leg.clone(), leg],
            }),
        },
        Command::RemoveDart {
            dart: DartKey::new(0, 0),
        },
        Command::AddSymmetry {
            identity: Identity::New,
            symmetry: Symmetry {
                axis: (point, point),
                kind: SymmetryKind::Fold,
            },
        },
        Command::RemoveSymmetry {
            symmetry: SymmetryKey::new(0, 0),
        },
    ]
}

#[test]
fn the_edits_of_this_phase_are_shape_and_metadata() {
    let doc = block::trouser_front();
    let classes: Vec<ChangeClass> = implemented(&doc)
        .iter()
        .map(Command::class)
        .filter(|class| *class == ChangeClass::Shape)
        .collect();
    assert_eq!(classes.len(), 5);
    for command in implemented(&doc) {
        assert!(
            matches!(command.class(), ChangeClass::Shape | ChangeClass::Metadata),
            "{command:?}"
        );
    }
}

#[test]
fn every_edit_that_changes_the_nodes_is_topology() {
    let doc = block::trouser_front();
    let commands = topology(&doc);
    assert_eq!(commands.len(), 15);
    for command in commands {
        assert_eq!(command.class(), ChangeClass::Topology, "{command:?}");
    }
}

#[test]
fn pinning_costs_the_derivation_nothing() {
    let doc = block::trouser_front();
    let piece = front(&doc);
    let commands = [
        Command::SetPin {
            identity: Identity::New,
            pin: Pin {
                piece,
                rest: [0.0, 0.0],
                to: [0.0, 0.0, 0.0],
            },
        },
        Command::ClearPin {
            pin: PinKey::new(0, 0),
        },
    ];
    for command in commands {
        assert_eq!(command.class(), ChangeClass::Sim, "{command:?}");
    }
}

#[test]
fn an_edit_whose_tool_has_not_arrived_is_an_error_not_a_panic() {
    let mut doc = block::trouser_front();
    let piece = front(&doc);
    let point = node(&doc, "cintura_cf");
    for command in [
        Command::RemoveNode { piece, node: point },
        Command::RemovePiece { piece },
        Command::SetSamples {
            piece,
            node: point,
            to: 8,
        },
    ] {
        assert_eq!(
            command.clone().apply(&mut doc),
            Err(DocError::NotYetImplemented),
            "{command:?}"
        );
    }
}

#[test]
fn a_restored_point_keeps_every_reference_to_it() {
    let mut doc = block::trouser_front();
    let piece = front(&doc);
    let hip = node(&doc, "cadera_lat");
    let knee = node(&doc, "rodilla_lat");
    let seam = doc.seams.insert(Seam::plain(
        EdgeRange::between(piece, hip, knee),
        EdgeRange::between(piece, hip, knee),
        SeamOrientation::Opposed,
    ));

    let value = doc.points.remove(hip).expect("the key is live");
    assert!(doc.points.get(hip).is_none());
    doc.points.restore(hip, value).expect("the slot is free");

    let held = doc.seams.get(seam).expect("the seam is live");
    assert_eq!(held.a.head.from, hip);
    assert!(doc.points.get(hip).is_some());
    assert_eq!(doc.label_of(piece, hip).as_deref(), Some("cadera_lat"));
}

#[test]
fn a_command_on_an_empty_document_is_an_error_not_a_panic() {
    let mut doc = Doc::new(MeasureSet::default());
    let command = Command::MovePoint {
        point: PointKey::new(0, 0),
        to: [Binding::literal(0.0), Binding::literal(0.0)],
    };
    assert!(command.apply(&mut doc).is_err());
}
