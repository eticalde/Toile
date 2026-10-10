#![allow(
    clippy::float_cmp,
    reason = "a notch, an offset and a depth are numbers the file writes and the product keeps"
)]

use std::collections::BTreeSet;

use toile_doc::{Doc, Grain, LineVertex, PointKey, Segment, Winding};

use super::{pattern, product};

#[test]
fn every_piece_is_named_as_the_owner_named_it_in_file_order() {
    let product = product();
    let names: Vec<&str> = product
        .doc
        .pieces
        .iter()
        .map(|(_, p)| p.name.as_str())
        .collect();
    let owner: Vec<String> = pattern()
        .blocks
        .iter()
        .flat_map(|block| &block.pieces)
        .map(|piece| piece.name.clone())
        .collect();
    assert_eq!(names, owner);
    assert_eq!(names.len(), 10);
}

#[test]
fn the_contours_count_the_corners_and_curves_the_outlines_draw() {
    let product = product();
    let count = |name: &str| {
        let key = product.doc.piece_named(name).expect("imported");
        let piece = product.doc.pieces.get(key).expect("live");
        let curves = piece.contour.iter().filter(|n| n.segment.bends()).count();
        (piece.contour.len(), curves)
    };
    assert_eq!(count("DELANTERO"), (10, 3));
    assert_eq!(count("TRASERO + CANESU"), (12, 1));
    assert_eq!(count("PRETINA"), (6, 0));
    assert_eq!(count("VISTA BRAGUETA"), (5, 1));
    assert_eq!(count("SOLAPA TRASERA"), (6, 2));
    for small in [
        "BOLSA PILL DELANTERA",
        "BOLSA PILL TRASERA",
        "TIRA PASACINTOS",
    ] {
        assert_eq!(count(small), (4, 0), "{small}");
    }
    let corners: usize = product.report.pieces.iter().map(|p| p.nodes).sum();
    let curves: usize = product.report.pieces.iter().map(|p| p.curves.len()).sum();
    // Every point of the product is drawn by something: a corner, a corner's
    // handle, a place of an internal line, or one of that line's handles. The
    // contours alone no longer account for them all, and nothing is stray.
    let mut cited: BTreeSet<PointKey> = BTreeSet::new();
    for (_, piece) in product.doc.pieces.iter() {
        for node in &piece.contour {
            cited.insert(node.point);
            cited.extend(node.segment.handles().into_iter().flat_map(|(a, b)| [a, b]));
        }
    }
    assert_eq!(cited.len(), corners + 2 * curves);
    for (_, line) in product.doc.lines.iter() {
        cited.extend(line.vertices().filter_map(LineVertex::point));
        cited.extend(line.handles());
    }
    assert_eq!(cited.len(), product.doc.points.len());
}

#[test]
fn every_corner_shows_the_name_of_the_point_it_stands_on() {
    let product = product();
    for (key, piece) in product.doc.pieces.iter() {
        for point in piece.anchors() {
            let label = product
                .doc
                .label_of(key, point)
                .expect("a corner of the piece");
            assert!(
                !label.starts_with('P') || label.len() > 3,
                "{}: {label}",
                piece.name
            );
        }
    }
    let flap = product.doc.piece_named("VISTA BRAGUETA").expect("imported");
    assert!(
        product.doc.shows_label(flap, "fv_4").is_some(),
        "an arc's end takes its point's name"
    );
}

#[test]
fn the_waistband_carries_its_two_notches_at_the_corners_the_file_cuts_them() {
    let product = product();
    let band = product.doc.piece_named("PRETINA").expect("imported");
    assert_eq!(product.doc.notches.len(), 2);
    let mut at: Vec<String> = product
        .doc
        .notches
        .iter()
        .map(|(_, notch)| {
            assert_eq!(notch.at.piece, band);
            assert_eq!(notch.at.t, 0.0);
            product.doc.label_of(band, notch.at.from).expect("a corner")
        })
        .collect();
    at.sort();
    assert_eq!(at, ["wb_cf", "wb_side"]);
}

/// The grain is the vertical because the file says nothing else: all ten
/// `<grainline>` elements carry an empty rotation, which is a piece nobody
/// put a grain line on and not an angle to read.
#[test]
fn every_piece_runs_clockwise_on_the_vertical_grain_where_the_overview_lays_it() {
    let product = product();
    let written = pattern();
    let read = written.blocks.iter().flat_map(|block| &block.pieces);
    assert!(read.clone().all(|piece| piece.grain.is_none()));
    assert_eq!(read.count(), 10);
    for (_, piece) in product.doc.pieces.iter() {
        assert_eq!(piece.winding, Winding::Cw, "{}", piece.name);
        assert_eq!(piece.grain, Grain::VERTICAL);
        assert_eq!(piece.placement, None, "{}", piece.name);
    }
    for note in &product.report.pieces {
        assert_eq!(
            note.placement,
            [0.0, 0.0],
            "the owner never moved {}",
            note.name
        );
        assert_eq!(note.grain, None, "the owner drew no grain on {}", note.name);
    }
}

#[test]
fn a_straight_spline_is_a_straight_tract_and_a_curve_is_flattened_finely_enough() {
    let product = product();
    for note in &product.report.pieces {
        for (tract, samples, stray) in &note.curves {
            assert!(*stray <= 0.01, "{}: {tract} strays {stray} cm", note.name);
            assert!(*samples >= 2, "{tract}");
        }
    }
    let front = product.doc.piece_named("DELANTERO").expect("imported");
    let held = product.doc.pieces.get(front).expect("live");
    let straight = held
        .contour
        .iter()
        .filter(|n| n.segment == Segment::Line)
        .count();
    assert_eq!(
        straight, 7,
        "the zero-handled splines and the leg are straight"
    );
}

/// An along-line point on a horizontal or vertical line takes the sense that
/// line has for the imported body: the waistband's two corners, and the places
/// of the internal lines drawn on the waistband and the chain strip.
#[test]
fn the_along_line_points_that_take_their_direction_from_the_body_are_the_waistband_s_and_its_lines()
{
    assert_eq!(
        product().report.directions,
        [
            "wb_cf", "wb_side", "wb_l1a", "wb_l1b", "wb_l2a", "wb_l2b", "wb_l3a", "wb_l3b",
            "wb_l4a", "cs_u1", "cs_u2", "cs_g0"
        ]
    );
}

#[test]
fn the_product_reads_back_to_the_very_bytes_it_was_written_as() {
    let product = product();
    let written = product.doc.to_canonical_json();
    let read = Doc::from_json(&written).expect("the product is a Toile file");
    assert_eq!(read, product.doc);
    assert_eq!(read.to_canonical_json(), written);
}

#[test]
fn every_internal_path_seam_allowance_and_label_is_reported() {
    let report = product().report;
    let internal: usize = report.pieces.iter().map(|p| p.internal.len()).sum();
    assert_eq!(internal, 25);
    let allowances: Vec<Option<f64>> = report.pieces.iter().map(|p| p.seam_allowance).collect();
    assert_eq!(allowances.iter().filter(|a| a.is_none()).count(), 2);
    assert!(
        report
            .pieces
            .iter()
            .all(|p| !p.labels.is_empty() && !p.letter.is_empty())
    );
    let band = report
        .pieces
        .iter()
        .find(|p| p.name == "PRETINA")
        .expect("imported");
    assert!(band.on_fold);
    assert_eq!(band.notches.len(), 2);
    assert!(
        band.notches
            .iter()
            .all(|n| n.kind == "slit" && n.length == 0.5)
    );
}

/// Nothing the file says about cutting a piece out is dropped on the way into
/// the document: the allowance, the count, the letter and the label's lines
/// arrive as the file writes them, and the report keeps saying the same.
///
/// The numbers are the owner's own, measured off the fixture: the waistband
/// is cut once at 1.5 cm, the two strips are cut on their line, and the back
/// flap's label says two where the garment wants four — the document holds
/// the phrase and the count side by side and reads neither into the other.
#[test]
fn the_document_keeps_what_the_file_says_about_cutting_each_piece_out() {
    let product = product();
    let written = pattern();
    for piece in written.blocks.iter().flat_map(|block| &block.pieces) {
        let key = product.doc.piece_named(&piece.name).expect("imported");
        let held = product.doc.pieces.get(key).expect("live");
        assert_eq!(held.seam_allowance, piece.seam_allowance, "{}", piece.name);
        assert_eq!(held.quantity, piece.quantity, "{}", piece.name);
        assert_eq!(held.letter.as_deref(), Some(piece.letter.as_str()));
        assert_eq!(held.labels, piece.labels, "{}", piece.name);
        assert!(held.says_how_it_is_cut(), "{}", piece.name);
    }
    let cut = |name: &str| {
        let key = product.doc.piece_named(name).expect("imported");
        let held = product.doc.pieces.get(key).expect("live");
        (
            held.letter.clone(),
            held.quantity,
            held.seam_allowance,
            held.labels.clone(),
        )
    };
    assert_eq!(
        cut("PRETINA"),
        (
            Some("C".to_owned()),
            1,
            Some(1.5),
            vec!["PRETINA".to_owned(), "cortar 1 al doblez c.b.".to_owned()]
        )
    );
    assert_eq!(
        cut("SOLAPA TRASERA"),
        (
            Some("J".to_owned()),
            2,
            Some(1.0),
            vec![
                "SOLAPA TRASERA".to_owned(),
                "cortar 2 + entretela".to_owned()
            ]
        )
    );
    for strip in ["TIRA PASACINTOS", "TIRA CADENA"] {
        let (_, quantity, allowance, _) = cut(strip);
        assert_eq!((quantity, allowance), (1, None), "{strip}");
    }
}
