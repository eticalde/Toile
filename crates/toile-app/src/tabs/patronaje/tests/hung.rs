use eframe::egui::epaint::{ClippedShape, ColorMode, Shape};
use eframe::egui::{Pos2, Rect};
use toile_engine::couture::HOLDS_ITS_RATIO;
use toile_engine::draft::{
    Command, Doc, EdgeRange, Elastic, Hang, Identity, PieceKey, PointKey, block,
};

use super::super::layout;
use super::super::state::Scope;
use super::studio::{Studio, painted};

/// The shipped front alone on the mat, where its tracts are drawn.
fn detail(doc: Doc) -> (Studio, PieceKey) {
    let mut studio = Studio::new(doc);
    let piece = studio.doc().piece_keys()[0];
    studio.state.scope = Scope::Piece;
    studio.state.active = Some(piece);
    studio.frame(Vec::new());
    (studio, piece)
}

/// The hooks the last frame drew in the ink a hang is marked with, each as the
/// two ends of one stroke.
fn hooks(studio: &Studio) -> Vec<[Pos2; 2]> {
    painted(studio)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::LineSegment { points, stroke } if stroke.color == studio.theme.hang => {
                Some(*points)
            }
            _ => None,
        })
        .collect()
}

/// The first two nodes of the piece, in contour order.
fn first_tract(studio: &Studio, piece: PieceKey) -> (PointKey, PointKey) {
    let nodes = studio
        .session
        .draft()
        .expect("a product is open")
        .points_cm(piece);
    (nodes[0].0, nodes[1].0)
}

/// Hangs the piece's first tract from the body's waist and draws the frame
/// after it.
fn hang_the_first_tract(studio: &mut Studio, piece: PieceKey) -> (PointKey, PointKey) {
    let (from, to) = first_tract(studio, piece);
    let at = EdgeRange::between(piece, from, to);
    studio
        .session
        .edit(Command::AddHang {
            identity: Identity::New,
            hang: Hang::new(at, Hang::WAIST),
        })
        .expect("both ends are nodes of the piece");
    studio.frame(Vec::new());
    (from, to)
}

/// A tract the body holds up is told from one it does not, without anything
/// being selected or pointed at.
///
/// The same argument the tape makes for itself: a hang is nowhere in the
/// outline — the line is cut the same either way — so a product opened six
/// months later would say nothing about what holds it on the person.
#[test]
fn the_mat_combs_a_hung_tract_and_leaves_every_other_bare() {
    let (mut studio, piece) = detail(block::trouser_front());
    assert!(hooks(&studio).is_empty(), "a plain contour wears no hooks");
    let (from, to) = hang_the_first_tract(&mut studio, piece);

    let combed = hooks(&studio);
    assert!(combed.len() >= 2, "a comb, not a mark: {}", combed.len());
    let ends = [from, to].map(|node| studio.on_glass(node));
    // Each hook crosses the tract square: its middle is on the line between the
    // two nodes, and the stroke itself runs across that line rather than along
    // it. A tract with a bend in it would fail the first of those, because a
    // hook is square to the segment under it and not to the chord.
    let run = ends[1] - ends[0];
    for hook in &combed {
        let middle = hook[0] + (hook[1] - hook[0]) / 2.0;
        let along = (middle - ends[0]).dot(run) / run.length_sq();
        assert!(
            (-0.01..=1.01).contains(&along),
            "{middle:?} is past the ends of the tract"
        );
        let off = middle.distance(ends[0] + run * along);
        assert!(off < 0.5, "{middle:?} is {off} points off the tract");
        let across = hook[1] - hook[0];
        assert!(across.dot(run).abs() < 0.01, "{across:?} runs along it");
    }
    let first = combed[0][0] + (combed[0][1] - combed[0][0]) / 2.0;
    assert!(
        first.distance(ends[0]) < 0.5,
        "the comb starts at the node the run leaves: {first:?} against {:?}",
        ends[0]
    );
}

/// A tract that is hung and elasticated wears both marks, and they cannot be
/// read for each other.
///
/// This is the case that decides the drawing: a waistband is a stretch held in
/// by an elastic and held up by a ring, written over the very same run. Two
/// bands down one line would leave whichever went second as the only one
/// anybody could see, so the hang crosses the line the tape lies along and
/// reaches past it on both sides.
#[test]
fn a_tract_held_in_and_held_up_wears_a_band_and_a_comb_over_it() {
    let (mut studio, piece) = detail(block::trouser_front());
    let (from, to) = hang_the_first_tract(&mut studio, piece);
    studio
        .session
        .edit(Command::AddElastic {
            identity: Identity::New,
            elastic: Elastic::new(EdgeRange::between(piece, from, to), 0.85, HOLDS_ITS_RATIO),
        })
        .expect("both ends are nodes of the piece");
    studio.frame(Vec::new());

    let tapes: Vec<&Shape> = painted(&studio)
        .into_iter()
        .filter(|shape| {
            matches!(shape, Shape::Path(path)
                if path.stroke.color == ColorMode::Solid(studio.theme.elastic))
        })
        .collect();
    assert_eq!(tapes.len(), 1, "one tract held in, one tape");
    let Shape::Path(tape) = tapes[0] else {
        unreachable!("the filter kept paths only")
    };
    let combed = hooks(&studio);
    assert!(!combed.is_empty(), "and the hooks are still drawn");
    assert_ne!(
        studio.theme.hang, studio.theme.elastic,
        "two marks, two inks"
    );
    // Measured against what was painted and not against a constant: the hooks
    // have to stand out past the band, whatever width the band is drawn at.
    let hook = combed[0][0].distance(combed[0][1]);
    assert!(
        hook > tape.stroke.width,
        "a hook of {hook} points cannot be seen across a band of {}",
        tape.stroke.width
    );
}

/// And the comb is there on the whole product too, moved with the piece.
///
/// That is the view a product opens on, so a hang invisible there would be
/// invisible until somebody happened to open the right piece.
#[test]
fn the_whole_product_wears_the_comb_where_it_lays_the_piece() {
    let (mut studio, piece) = detail(block::trousers());
    let (from, _) = hang_the_first_tract(&mut studio, piece);
    studio.state.scope = Scope::Product;
    studio.frame(Vec::new());

    let combed = hooks(&studio);
    assert!(!combed.is_empty(), "one tract hung, one comb");
    let draft = studio.session.draft().expect("a product is open");
    let shift = layout::of(draft)
        .into_iter()
        .find(|it| it.piece == piece)
        .expect("the piece is laid out")
        .shift;
    let cm = draft.resolved(from).expect("the node resolves");
    let head = studio
        .state
        .view
        .to_screen([cm[0] + shift[0], cm[1] + shift[1]]);
    let first = combed[0][0] + (combed[0][1] - combed[0][0]) / 2.0;
    assert!(
        first.distance(head) < 0.5,
        "the comb starts at {first:?}, the tract at {head:?}"
    );
}

/// The mat's own shapes, the panels around it left out.
///
/// Scoped by the clip rect the drawing is painted under, because the tab's bars
/// answer to more than the document: the arrow that steps back through the
/// history lights the moment there is anything to step back through, and this
/// test is about the drawing.
fn on_the_mat(shapes: &[ClippedShape], mat: Rect) -> Vec<Shape> {
    shapes
        .iter()
        .filter(|clipped| clipped.clip_rect == mat)
        .map(|clipped| clipped.shape.clone())
        .collect()
}

/// The clip rect the mat paints under, read off a mark only the mat draws.
fn mat_of(studio: &Studio) -> Rect {
    studio
        .shapes
        .iter()
        .find(|clipped| {
            matches!(&clipped.shape, Shape::LineSegment { stroke, .. }
                if stroke.color == studio.theme.hang)
        })
        .expect("the hung tract is on the drawing")
        .clip_rect
}

/// A product nobody hung is drawn exactly as it was drawn before hangs existed.
///
/// Proven rather than asserted, and proven the only way a drawing can be: the
/// frame before the hang and the frame after it is taken off again paint the
/// same shapes in the same order, down to the last stroke. An empty comb, a
/// stroke of no length or a mark drawn in nothing would all show up here.
#[test]
fn a_product_hung_from_nothing_is_drawn_exactly_as_it_was() {
    let (mut studio, piece) = detail(block::trouser_front());
    let bare = studio.shapes.clone();
    hang_the_first_tract(&mut studio, piece);
    let mat = mat_of(&studio);

    studio.session.undo().expect("the step back");
    studio.frame(Vec::new());
    assert!(
        studio.doc().hangs.is_empty(),
        "the hang is off the document"
    );
    assert!(hooks(&studio).is_empty(), "and off the drawing");
    assert_eq!(
        on_the_mat(&studio.shapes, mat),
        on_the_mat(&bare, mat),
        "the mat drew something it did not draw before"
    );
}
