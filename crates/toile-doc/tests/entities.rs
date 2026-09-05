#![allow(missing_docs, reason = "a test crate publishes no API surface")]
#![allow(
    clippy::float_cmp,
    reason = "a file reads back the very numbers it was written from"
)]

use toile_doc::{
    Binding, Dart, Doc, EdgeAnchor, EdgeRange, FoldDirection, Grain, MeasureSet, Notch, NotchCount,
    Piece, PieceKey, Pin, Point, PointKey, Seam, SeamKind, SeamOrientation, Segment, Symmetry,
    SymmetryKind, Variable, Winding,
};

/// A document carrying one of every entity the model declares.
///
/// Nothing draws most of these yet. The file format has to cover them all the
/// same: a field added later is a default, and an entity added later is a
/// migration.
fn everything() -> Doc {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
    doc.mannequins
        .insert(MeasureSet::new("Talla 42", [("cintura", 88.0)]));
    doc.variables
        .insert(Variable::new("holgura_cadera", Binding::literal(1.0)));

    let points: Vec<PointKey> = (0..8)
        .map(|i| {
            doc.points
                .insert(Point::at(f64::from(i), f64::from(i) * 2.0))
        })
        .collect();
    let handles = (
        doc.points.insert(Point::at(0.5, 0.25).named("manija_sale")),
        doc.points.insert(Point::at(1.5, 1.25)),
    );

    let front = doc.pieces.insert(Piece::polygon(
        "Delantero",
        points[..4].to_vec(),
        Winding::Cw,
    ));
    let back = doc.pieces.insert(Piece::polygon(
        "Trasero",
        points[4..].to_vec(),
        Winding::Ccw,
    ));
    let held = doc.pieces.get_mut(front).expect("the key is live");
    held.contour[1].segment = Segment::Cubic {
        out: handles.0,
        into: handles.1,
    };
    held.contour[1].samples = 24;
    held.grain = Grain::Angle(0.25);

    seams(&mut doc, front, back, &points);
    marks(&mut doc, front, back, &points);
    doc.symmetries.insert(Symmetry {
        axis: (points[0], points[3]),
        kind: SymmetryKind::Mirror,
    });
    doc.pins.insert(Pin {
        piece: back,
        rest: [12.5, -3.25],
        to: [0.0, 1.0, 0.5],
    });
    doc
}

fn seams(doc: &mut Doc, front: PieceKey, back: PieceKey, points: &[PointKey]) {
    let side = EdgeRange::between(front, points[1], points[2]);
    let facing = EdgeRange {
        head: EdgeAnchor {
            piece: back,
            from: points[5],
            t: 0.25,
        },
        tail: EdgeAnchor::at_node(back, points[6]),
    };
    doc.seams
        .insert(Seam::plain(side, facing, SeamOrientation::Opposed));
    doc.seams.insert(Seam {
        kind: SeamKind::Eased { expected_cm: 1.5 },
        tolerance: Some(0.75),
        ..Seam::plain(side, facing, SeamOrientation::Aligned)
    });
    doc.seams.insert(Seam {
        kind: SeamKind::Gathered { ratio: 2.0 },
        ..Seam::plain(side, facing, SeamOrientation::Aligned)
    });
}

fn marks(doc: &mut Doc, front: PieceKey, back: PieceKey, points: &[PointKey]) {
    let here = doc.notches.insert(Notch {
        at: EdgeAnchor {
            piece: front,
            from: points[1],
            t: 0.5,
        },
        mate: None,
        count: NotchCount::Double,
    });
    let there = doc.notches.insert(Notch {
        at: EdgeAnchor::at_node(back, points[5]),
        mate: Some(here),
        count: NotchCount::Triple,
    });
    doc.notches.get_mut(here).expect("the key is live").mate = Some(there);
    let seam = doc.seams.keys().next().expect("the document sews one");
    doc.darts.insert(Dart {
        apex: points[2],
        legs: (points[1], points[3]),
        seam,
        fold: FoldDirection::TowardStart,
    });
}

#[test]
fn every_entity_the_model_declares_survives_the_file() {
    let doc = everything();
    let written = doc.to_canonical_json();
    let read = Doc::from_json(&written).expect("what the writer wrote, the reader reads");
    assert_eq!(read, doc);
    assert_eq!(read.to_canonical_json(), written);
}

#[test]
fn a_kind_is_written_as_a_tag_beside_the_fields_it_carries() {
    let written = everything().to_canonical_json();
    for shape in [
        "\"kind\": \"cubic\"",
        "\"out\": \"8.0\"",
        "\"into\": \"9.0\"",
        "\"samples\": 24",
        "\"kind\": \"plain\"",
        "\"kind\": \"eased\"",
        "\"expected_cm\": 1.5",
        "\"kind\": \"gathered\"",
        "\"ratio\": 2",
        "\"orientation\": \"opposed\"",
        "\"count\": \"double\"",
        "\"count\": \"triple\"",
        "\"fold\": \"toward_start\"",
        "\"kind\": \"mirror\"",
        "\"winding\": \"ccw\"",
        "\"radians\": 0.25",
        "\"tolerance\": 0.75",
        "\"t\": 0.25",
    ] {
        assert!(written.contains(shape), "the file never says {shape}");
    }
}

#[test]
fn a_pin_keeps_the_place_it_holds_and_the_place_it_holds_it_to() {
    let written = everything().to_canonical_json();
    let read = Doc::from_json(&written).expect("what the writer wrote, the reader reads");
    let (_, pin) = read.pins.iter().next().expect("the document pins one");
    assert_eq!(pin.rest, [12.5, -3.25]);
    assert_eq!(pin.to, [0.0, 1.0, 0.5]);
}
