#![allow(missing_docs, reason = "a test crate publishes no API surface")]
#![allow(
    clippy::float_cmp,
    reason = "an unfolded piece hands back the very numbers the drawn one did"
)]

/// The waistband the draft's own formulas write, and the fold put on it.
mod band;
/// What an elastic and a hang written on a folded piece come to hold.
mod held;

use toile_engine::demo;
use toile_engine::draft::{Draft, block};
use toile_engine::export::{A4, piece_to_pdf, to_svg};

use crate::band::{drafted, width};

/// The number the whole card turns on: the drawn half is 47.5 centimetres wide
/// and the cloth it is cut from is 95.
///
/// Not 91, which is what the band measures drawn whole — the four centimetres
/// of button extension past centre front are mirrored along with everything
/// else, so a fold puts an extension on the far side as well. A fold reflects
/// the drawing; it cannot leave one end of it out. That is the very trap a
/// draft that carries this piece on the fold pays for at the cutting table, and
/// the reason to draw this particular band whole. The concept is still the
/// right one: what the fold owes is that Toile sees the cloth, and it does.
#[test]
fn folding_the_waistband_doubles_the_width_it_is_cut_at() {
    let (drawn, piece) = drafted(false);
    assert!((width(drawn.cloth_cm(piece)) - 47.5).abs() < 1.0e-9);
    assert_eq!(drawn.cloth_cm(piece), drawn.flat_cm(piece));
    assert_eq!(drawn.cloth(piece), None);

    let (folded, piece) = drafted(true);
    assert!(
        (width(folded.cloth_cm(piece)) - 95.0).abs() < 1.0e-9,
        "{} cm",
        width(folded.cloth_cm(piece))
    );
    assert!(
        (width(folded.flat_cm(piece)) - 47.5).abs() < 1.0e-9,
        "the drawing is still the half"
    );
    // The crease is handed back tail first, the way the cloth's own outline
    // opens on it.
    let crease = folded.cloth(piece).map(|cloth| cloth.axis);
    assert_eq!(crease, Some([[83.5, 11.0], [83.5, 7.0]]));
}

/// The drawn contour keeps every node it had, and the cloth keeps every sample
/// of the half that is not the crease, twice over minus the two ends.
#[test]
fn the_cloth_is_the_drawn_walk_and_its_mirror_and_nothing_of_the_crease() {
    let (folded, piece) = drafted(true);
    assert_eq!(folded.points_cm(piece).len(), 6, "six nodes, as drawn");
    // Five of the six tracts are cloth; the sixth is the crease. The walk
    // carries its four interior nodes and both ends of the axis, and the mirror
    // carries the four interior ones again.
    assert_eq!(folded.cloth_cm(piece).len(), 6 + 4);
    let perimeter = folded.perimeter_cm(piece);
    assert!(
        (perimeter - 2.0 * (47.5 + 4.0 + 47.5)).abs() < 1.0e-9,
        "{perimeter} cm"
    );
}

/// The mesher and the drape take the cloth, so a folded piece drapes as the
/// piece it is. The witness is the mesh's own width: half a band would give a
/// panel of cloth half a metre across where the garment needs most of one.
#[test]
fn the_mesher_is_handed_the_whole_cloth() {
    let (folded, piece) = drafted(true);
    let outline = folded.outline_m(piece);
    assert!(
        (width(outline) - 0.95).abs() < 1.0e-9,
        "{} m",
        width(outline)
    );
    let pipeline = demo::pipeline(outline);
    let mesh = width(&pipeline.pos2d);
    assert!((mesh - 0.95).abs() < 1.0e-3, "{mesh} m of mesh");
    // And the boundary a seam or an elastic is read onto is the cloth's, so it
    // carries more vertices than the drawing would have given it.
    let (drawn, _) = drafted(false);
    let half = demo::pipeline(drawn.outline_m(piece));
    assert!(
        pipeline.n_boundary() > half.n_boundary(),
        "{} against {}",
        pipeline.n_boundary(),
        half.n_boundary()
    );
}

/// The sheet of paper carries the cloth, at true scale, with every mark drawn
/// on both halves. A cutter given the drawn half would have to lay the paper on
/// a fold, and the cut line and the net line are two different places to lay
/// it.
#[test]
fn the_exported_sheet_is_the_cloth_and_carries_both_halves_of_a_mark() {
    let (folded, _) = drafted(true);
    let drawing = to_svg(&folded).expect("the band draws").text;
    // The far edge of the mirrored half, in millimetres: 36 cm mirrored about
    // 83.5 lands at 131.
    assert!(drawing.contains("1310.00"), "{drawing}");
    assert!(drawing.contains("width=\"970.00mm\""), "{drawing}");
    // The belt-loop mark at 50 cm, and its mirror at 117.
    assert!(
        drawing.contains("M 500.00 70.00 L 500.00 110.00"),
        "{drawing}"
    );
    assert!(
        drawing.contains("M 1170.00 70.00 L 1170.00 110.00"),
        "{drawing}"
    );

    // Drawn whole, the same mark is on the sheet once and only once.
    let (drawn, _) = drafted(false);
    let flat = to_svg(&drawn).expect("the band draws").text;
    assert_eq!(flat.matches("M 500.00 70.00").count(), 1, "{flat}");
    assert!(!flat.contains("1170.00"), "{flat}");
}

/// The paper is counted for what the cloth measures, not for what the drawn
/// half does.
///
/// Three sheets of A4 hold the drawn half and six hold the band, so a printer
/// counting the half would hand back something a person could tape up, cut and
/// sew, and only then find to be half a waistband. The sheet says the
/// ninety-five centimetres out loud as well, in the one line a ruler can check.
#[test]
fn the_sheets_are_counted_for_the_cloth_and_not_for_the_drawn_half() {
    let (folded, piece) = drafted(true);
    let whole =
        piece_to_pdf(&folded, piece, A4).expect("a band ninety-five centimetres wide tiles");
    assert_eq!(whole.piles[0].grid, [6, 1]);
    assert_eq!(whole.sheets(), 6);
    let said = String::from_utf8_lossy(&whole.bytes).into_owned();
    assert!(
        said.contains("La pieza entera mide 95.0 cm de ancho"),
        "{said}"
    );

    let (drawn, piece) = drafted(false);
    let half = piece_to_pdf(&drawn, piece, A4).expect("the drawn half tiles too");
    assert_eq!(half.piles[0].grid, [3, 1]);
    assert_eq!(drawn.cloth_cm(piece).len(), 6);
}

/// A piece nobody folded is what it was before folds existed, through every
/// path the unfold touches: the flattening, the metres the mesher takes, the
/// perimeter, and the bytes of the sheet.
#[test]
fn a_piece_with_no_axis_is_bit_identical_through_every_path() {
    let draft = Draft::from_doc(block::trousers()).expect("the block resolves");
    for piece in draft.doc().piece_keys() {
        assert_eq!(draft.cloth(piece), None);
        assert_eq!(draft.cloth_cm(piece), draft.flat_cm(piece));
        let metres: Vec<[f64; 2]> = draft
            .flat_cm(piece)
            .iter()
            .map(|&at| toile_engine::draft::to_metres(at))
            .collect();
        assert_eq!(draft.outline_m(piece), metres.as_slice());
        let cum = draft.node_cum(piece);
        assert_eq!(draft.perimeter_cm(piece), cum[cum.len() - 1]);
        let nodes = draft.points_cm(piece);
        let at = toile_engine::draft::EdgeAnchor {
            piece,
            from: nodes[1].0,
            t: 0.25,
        };
        let fraction = draft
            .anchor_fraction(&at)
            .expect("the node is on the piece");
        let along = cum[1] + (cum[2] - cum[1]) * 0.25;
        assert_eq!(fraction, along / cum[cum.len() - 1]);
    }
}

/// A place on the crease is not on the cloth's boundary at all, so nothing can
/// be sewn or held in there. It is said with a `None` rather than a fraction
/// that would land somewhere on the mirrored half.
#[test]
fn a_place_on_the_crease_anchors_to_nothing() {
    let (folded, piece) = drafted(true);
    let node = |label: &str| {
        folded
            .doc()
            .shows_label(piece, label)
            .expect("the band names it")
    };
    let at = |from, t| toile_engine::draft::EdgeAnchor { piece, from, t };
    // The crease runs from `wb_2` to `wb_cb`, so halfway along the tract
    // leaving `wb_2` is inside the fold.
    assert_eq!(folded.anchor_fraction(&at(node("wb_2"), 0.5)), None);
    // Its two ends are on the cloth: one opens the boundary, the other closes
    // the drawn walk halfway round it.
    assert_eq!(folded.anchor_fraction(&at(node("wb_cb"), 0.0)), Some(0.0));
    let head = folded
        .anchor_fraction(&at(node("wb_2"), 0.0))
        .expect("the other end of the crease is cloth");
    assert!((head - 0.5).abs() < 1.0e-9, "{head}");
}
