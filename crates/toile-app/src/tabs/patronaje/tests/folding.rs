use eframe::egui::Event;
use eframe::egui::epaint::Shape;
use toile_engine::draft::{
    Command, Doc, EdgeAnchor, EdgeRange, Identity, MeasureSet, Notch, Piece, PieceKey, Point,
    PointKey, Symmetry, Winding,
};

use super::super::state::{Scope, Selection, Tool};
use super::super::{chalk, layout, tract};
use super::bench::Bench;
use super::studio::{Studio, painted};

/// A hundred-centimetre square drawn against its own left side, as the document
/// holds it, with the nodes it was drawn from.
fn square() -> (Doc, PieceKey, Vec<PointKey>) {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
    let corners = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
    let points: Vec<PointKey> = corners
        .into_iter()
        .map(|[x, y]| doc.points.insert(Point::at(x, y)))
        .collect();
    let piece = doc
        .pieces
        .insert(Piece::polygon("Pretina", points.clone(), Winding::Cw));
    Command::AddSymmetry {
        identity: Identity::New,
        symmetry: Symmetry::fold(EdgeRange::between(piece, points[3], points[0])),
    }
    .apply(&mut doc)
    .expect("both ends are nodes of the square");
    (doc, piece, points)
}

/// The same square on the mat, open on its own detail.
fn folded() -> (Bench, PieceKey) {
    let (doc, piece, _) = square();
    let mut bench = Bench::new(doc);
    bench.state.scope = Scope::Piece;
    bench.state.active = Some(piece);
    bench.frame(Vec::new());
    (bench, piece)
}

/// The mat frames the whole cloth, so the half nobody drew is on screen from
/// the first frame rather than off the left edge of it.
#[test]
fn the_mat_opens_framed_on_the_cloth_and_not_on_the_drawing() {
    let (bench, piece) = folded();
    let draft = bench.session.draft().expect("a product is open");
    let seen = |at: [f64; 2]| {
        let on_glass = bench.state.view.to_screen(at);
        (0.0..1320.0).contains(&on_glass.x) && (0.0..780.0).contains(&on_glass.y)
    };
    assert!(seen([-100.0, 50.0]), "the mirrored half is on the glass");
    assert!(seen([100.0, 50.0]), "and so is the drawn one");
    assert_eq!(draft.cloth_cm(piece).len(), 6);
}

/// A press on the mirrored half opens nothing.
///
/// The mirror is drawn, not editable: it has no nodes of its own, no tracts of
/// its own and no places of its own, so every tool that would take something in
/// hand there finds nothing. The tracing tool is the sharp case — it is the one
/// that puts a place wherever the cloth is — and the cloth it asks about is the
/// drawn contour.
#[test]
fn a_press_on_the_half_nobody_drew_takes_nothing_in_hand() {
    let (mut bench, piece) = folded();
    bench.state.tool = Tool::Trace;
    let draft = bench.session.draft().expect("a product is open");
    let tracts = tract::of(draft, piece);
    assert!(tract::covers(&tracts, [50.0, 50.0]), "the drawing is cloth");
    assert!(
        !tract::covers(&tracts, [-50.0, 50.0]),
        "and the mirror is not the drawing"
    );

    let at = bench.state.view.to_screen([-50.0, 50.0]);
    bench.frame(vec![Event::PointerMoved(at)]);
    bench.click(at);
    assert_eq!(bench.session.revision(), 0, "nothing reached the document");
    assert_eq!(bench.state.selection, Selection::None);
}

/// How many cuts of a notch the last frame drew.
///
/// The ink of a cut line at the width a mark is stroked at: on this square
/// nothing else is drawn that way — the outline is one closed path, the crease
/// is in the ink of a measurement, and no line is drawn inside the piece.
fn cuts(studio: &Studio) -> usize {
    let ink = studio.theme.outline;
    painted(studio)
        .into_iter()
        .filter(|shape| {
            matches!(shape, Shape::LineSegment { stroke, .. }
                if stroke.color == ink && stroke.width == chalk::NOTCH_W)
        })
        .count()
}

/// A product open on one of its pieces, on a studio: the harness that keeps
/// what the mat painted.
fn shown(doc: Doc, piece: PieceKey) -> Studio {
    let mut studio = Studio::new(doc);
    studio.state.scope = Scope::Piece;
    studio.state.active = Some(piece);
    studio.frame(Vec::new());
    studio
}

/// A folded piece carries each of its notches twice — once on each side of the
/// fold — so the mat draws each of them twice.
///
/// The cut piece is what comes off the table: a mark shown only on the half
/// somebody drew is a pair of notches missing from the cloth that will be cut,
/// and the person laying the paper down has nothing to line it up on.
#[test]
fn the_mat_draws_every_notch_of_a_folded_piece_on_both_halves() {
    let (mut doc, piece, points) = square();
    for &from in &points[..2] {
        Command::AddNotch {
            identity: Identity::New,
            notch: Notch::lone(EdgeAnchor {
                piece,
                from,
                t: 0.5,
            }),
            mate: None,
        }
        .apply(&mut doc)
        .expect("the middle of a tract is a place on the contour");
    }
    let mut studio = shown(doc.clone(), piece);
    assert_eq!(cuts(&studio), 4, "two cut, two reflected");
    studio.state.scope = Scope::Product;
    studio.frame(Vec::new());
    assert_eq!(cuts(&studio), 4, "and the same on the whole product");

    // The same two marks with nothing to mirror across are drawn once each,
    // which is what says the count counts the marks and not the mat.
    let (key, _) = doc.symmetry_of(piece).expect("the square is folded");
    Command::RemoveSymmetry { symmetry: key }
        .apply(&mut doc)
        .expect("the axis is live");
    assert_eq!(cuts(&shown(doc, piece)), 2, "two cut, nothing reflected");
}

/// The overview gives a folded piece the room its cloth takes, and finds it
/// under the pointer over the whole of that room.
#[test]
fn the_overview_lays_out_and_picks_the_folded_piece_by_its_cloth() {
    let (mut bench, piece) = folded();
    bench.state.scope = Scope::Product;
    bench.frame(Vec::new());
    let laid = layout::of(bench.session.draft().expect("a product is open"));
    let bounds = layout::bounds(&laid).expect("the square has an outline");
    assert!(
        (f64::from(bounds.width()) - 200.0).abs() < 1.0e-6,
        "{bounds:?}"
    );
    let it = laid.iter().find(|it| it.piece == piece).expect("laid");
    let inside = [
        f64::from(bounds.left()) + 10.0,
        f64::from(bounds.top()) + 50.0,
    ];
    assert_eq!(
        layout::under(inside, &laid, 0.5).map(|found| found.piece),
        Some(piece),
        "the mirrored half is the piece too, at shift {:?}",
        it.shift
    );
}
