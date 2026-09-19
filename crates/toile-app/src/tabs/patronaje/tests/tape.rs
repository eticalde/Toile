use eframe::egui::epaint::{ColorMode, Shape};
use toile_engine::couture::HOLDS_ITS_RATIO;
use toile_engine::draft::{Command, Doc, EdgeRange, Elastic, Identity, PieceKey, PointKey, block};

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

/// The lines the last frame drew in the ink an elastic is marked with.
fn tapes(studio: &Studio) -> Vec<&Shape> {
    let ink = ColorMode::Solid(studio.theme.elastic);
    painted(studio)
        .into_iter()
        .filter(|shape| matches!(shape, Shape::Path(path) if path.stroke.color == ink))
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

/// Puts an elastic on the piece's first tract and draws the frame after it.
fn hold_the_first_tract(studio: &mut Studio, piece: PieceKey) -> (PointKey, PointKey) {
    let (from, to) = first_tract(studio, piece);
    studio
        .session
        .edit(Command::AddElastic {
            identity: Identity::New,
            elastic: Elastic::new(EdgeRange::between(piece, from, to), 0.85, HOLDS_ITS_RATIO),
        })
        .expect("both ends are nodes of the piece");
    studio.frame(Vec::new());
    (from, to)
}

/// A tract that pulls the cloth in is told from one that does not, without
/// anything being selected or pointed at.
///
/// The mark is the whole point of the section: an elastic is not visible in
/// the drawing — the outline is the same line either way — so a product opened
/// six months later would say nothing about what holds it on.
#[test]
fn the_mat_lays_a_tape_along_an_elastic_tract_and_along_no_other() {
    let (mut studio, piece) = detail(block::trouser_front());
    assert!(tapes(&studio).is_empty(), "a plain contour wears no tape");
    let (from, to) = hold_the_first_tract(&mut studio, piece);

    let laid = tapes(&studio);
    assert_eq!(laid.len(), 1, "one tract held, one tape");
    let Shape::Path(path) = laid[0] else {
        unreachable!("the filter kept paths only")
    };
    // Along the tract it names and not along some other: the tape starts where
    // the node it leaves is drawn and ends where the next one is.
    let ends = [from, to].map(|node| studio.on_glass(node));
    let drawn = [path.points[0], *path.points.last().expect("two ends")];
    for (drawn, wanted) in drawn.iter().zip(&ends) {
        assert!(
            drawn.distance(*wanted) < 0.5,
            "the tape runs {drawn:?}, the tract {wanted:?}"
        );
    }
}

/// And the tape is there on the whole product too, moved with the piece.
///
/// That is the view a product opens on, so an elastic invisible there would be
/// invisible until somebody happened to open the right piece.
#[test]
fn the_whole_product_wears_the_tape_where_it_lays_the_piece() {
    let (mut studio, piece) = detail(block::trousers());
    let (from, _) = hold_the_first_tract(&mut studio, piece);
    studio.state.scope = Scope::Product;
    studio.frame(Vec::new());

    let laid = tapes(&studio);
    assert_eq!(laid.len(), 1, "one tract held, one tape");
    let Shape::Path(path) = laid[0] else {
        unreachable!("the filter kept paths only")
    };
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
    assert!(
        path.points[0].distance(head) < 0.5,
        "the tape starts at {:?}, the tract at {head:?}",
        path.points[0]
    );
}
