/// Every refusal the gesture makes by hand, in the words the person reads.
mod refused;
/// What the bar says at each press, which is how the two gestures are told
/// apart.
mod said;

use eframe::egui::{Key, Modifiers, Pos2};
use toile_engine::draft::{
    Binding, Command, ContourNode, Doc, FoldDirection, Identity, PieceKey, Point, PointKey,
    SegmentEdit, block,
};

use super::super::dart::PUT_ON;
use super::super::dart::declare::{DECLARED, NODES};
use super::super::gesture::Gesture;
use super::super::state::{Scope, Selection, Tool};
use super::studio::Studio;

/// A wedge drawn into the back waistline by formula, the way a draft writes
/// one.
///
/// Every coordinate is an expression over the pattern's own quantities, which
/// is what a drafted back has and what no cut can produce: a cut writes the
/// place the pointer landed on. They go in after `cintura_cb`, on the straight
/// tract that runs to the side waist.
pub(super) const DRAWN: [(&str, &str, &str); NODES] = [
    ("pinza_a", "origen_tras + 4", "-levante_tras / 2"),
    ("pinza_pico", "origen_tras + 6", "altura_cadera / 2"),
    ("pinza_b", "origen_tras + 8", "0"),
];

/// The shipped trousers with that wedge drawn into the back, the back alone on
/// the mat, and the dart tool in hand.
pub(super) fn drafted() -> (Studio, PieceKey, [PointKey; NODES]) {
    drafted_from(DRAWN)
}

/// The same back, with `write` drawn into its waistline instead.
pub(super) fn drafted_from(
    write: [(&str, &str, &str); NODES],
) -> (Studio, PieceKey, [PointKey; NODES]) {
    let mut doc = block::trousers();
    let piece = doc
        .piece_named(block::BACK)
        .expect("the block draws a back");
    let nodes = drawn_in(&mut doc, piece, write);
    let mut studio = Studio::new(doc);
    studio.state.scope = Scope::Piece;
    studio.state.active = Some(piece);
    studio.frame(Vec::new());
    studio.key(Key::D, Modifiers::NONE);
    studio.frame(Vec::new());
    assert_eq!(studio.state.tool, Tool::Dart, "D takes the dart tool");
    (studio, piece, nodes)
}

/// Three nodes written into the waistline of `piece`, one after another.
pub(super) fn drawn_in(
    doc: &mut Doc,
    piece: PieceKey,
    write: [(&str, &str, &str); NODES],
) -> [PointKey; NODES] {
    let mut after = doc
        .shows_label(piece, "cintura_cb")
        .expect("the block names it");
    let mut made = Vec::with_capacity(NODES);
    for (label, x, y) in write {
        Command::InsertNode {
            piece,
            after: Some(after),
            identity: Identity::New,
            value: Point::at(binding(x), binding(y)).named(label),
            segment: SegmentEdit::Line,
            samples: 1,
        }
        .apply(doc)
        .expect("the node it follows is on the waistline");
        after = doc
            .shows_label(piece, label)
            .expect("the insertion named it");
        made.push(after);
    }
    [made[0], made[1], made[2]]
}

fn binding(source: &str) -> Binding {
    Binding::parse(source).expect("the block's own grammar")
}

/// The pointer pressing on each of `nodes` in turn, where the mat draws them.
pub(super) fn press_nodes(studio: &mut Studio, nodes: [PointKey; NODES]) {
    for node in nodes {
        let at = studio.on_glass(node);
        studio.click(at);
    }
    studio.frame(Vec::new());
}

/// What each of the three nodes is bound to, as the file spells it.
fn sources(studio: &Studio, nodes: [PointKey; NODES]) -> Vec<(String, String)> {
    let doc = studio.doc();
    nodes
        .iter()
        .map(|&key| {
            let held = doc.points.get(key).expect("the node is live");
            (held.x.source().into_owned(), held.y.source().into_owned())
        })
        .collect()
}

/// The contour of `piece` as the document holds it.
pub(super) fn contour(studio: &Studio, piece: PieceKey) -> Vec<ContourNode> {
    studio
        .doc()
        .pieces
        .get(piece)
        .expect("the key is live")
        .contour
        .clone()
}

/// The one dart the product holds.
fn only_dart(studio: &Studio) -> toile_engine::draft::Dart {
    let doc = studio.doc();
    assert_eq!(doc.darts.len(), 1, "one wedge, one dart");
    *doc.darts
        .iter()
        .next()
        .expect("the declaration wrote one")
        .1
}

/// Where a point of the document resolves to right now.
fn resolved(studio: &Studio, point: PointKey) -> [f64; 2] {
    studio
        .session
        .draft()
        .expect("a product is open")
        .resolved(point)
        .expect("the place resolves")
}

/// Where a fraction of the way from one node to another falls on the glass.
fn along(studio: &Studio, ends: (PointKey, PointKey), t: f64) -> Pos2 {
    let (from, to) = (resolved(studio, ends.0), resolved(studio, ends.1));
    studio
        .state
        .view
        .to_screen([0, 1].map(|axis| from[axis] + (to[axis] - from[axis]) * t))
}

/// Three presses on three nodes the contour already draws declare a dart over
/// them, and not one coordinate is rewritten.
///
/// That is the whole of why the gesture exists. A pattern drafted by formula
/// loses its draft the moment anything rewrites one of those coordinates, and
/// the only route open before this — three nodes deleted and cut again — does
/// exactly that, since a cut writes the place the pointer landed on.
#[test]
fn three_presses_on_a_drawn_wedge_declare_a_dart_and_rewrite_nothing() {
    let (mut studio, piece, nodes) = drafted();
    let was = contour(&studio, piece);
    let bound = sources(&studio, nodes);
    let points = studio.doc().points.len();
    press_nodes(&mut studio, nodes);

    let held = only_dart(&studio);
    assert_eq!(held.legs, (nodes[0], nodes[2]));
    assert_eq!(held.apex, nodes[1]);
    assert_eq!(held.fold, FoldDirection::TowardStart);
    assert!(
        studio.doc().seams.get(held.seam).is_some(),
        "and the thread that shuts it"
    );
    assert_eq!(contour(&studio, piece), was, "node for node");
    assert_eq!(
        studio.doc().points.len(),
        points,
        "not one point was placed"
    );
    assert_eq!(sources(&studio, nodes), bound, "nor one binding rewritten");
    assert_eq!(studio.session.undo_label(), Some(DECLARED));
    assert_eq!(studio.state.gesture, Gesture::Idle, "and let go of");
    assert_eq!(
        studio.state.selection,
        Selection::point(nodes[1]),
        "the panel opens on the dart the hand just declared"
    );
    assert_eq!(
        studio.state.refused, None,
        "the document was never asked for anything it refuses"
    );
}

/// Walked the other way round the contour the same three nodes make the same
/// wedge, pressed toward the other leg.
///
/// Which way a sewn dart lies is the one thing about it the drawing cannot say,
/// and the order of the presses is information the gesture already has — so it
/// is read the way the cut reads it, off the leg pressed first.
#[test]
fn the_wedge_walked_backward_is_pressed_toward_the_other_leg() {
    let (mut studio, _, nodes) = drafted();
    press_nodes(&mut studio, [nodes[2], nodes[1], nodes[0]]);
    let held = only_dart(&studio);
    assert_eq!(
        held.legs,
        (nodes[0], nodes[2]),
        "the wedge is written in contour order whichever way it was walked"
    );
    assert_eq!(held.apex, nodes[1]);
    assert_eq!(held.fold, FoldDirection::TowardEnd);
}

/// Undo takes the declaration back and leaves the wedge drawn: the three nodes
/// in the contour, each bound to the expression it was written with.
#[test]
fn undo_leaves_the_wedge_drawn_with_its_formulas_intact() {
    let (mut studio, piece, nodes) = drafted();
    let was = contour(&studio, piece);
    let bound = sources(&studio, nodes);
    let seams = studio.doc().seams.len();
    press_nodes(&mut studio, nodes);

    studio.session.undo().expect("the step back");
    studio.frame(Vec::new());
    assert!(studio.doc().darts.is_empty(), "the record is gone");
    assert_eq!(studio.doc().seams.len(), seams, "and its thread with it");
    assert_eq!(contour(&studio, piece), was, "node for node");
    assert_eq!(
        sources(&studio, nodes),
        bound,
        "and every coordinate is the expression it was drawn as"
    );
    assert_eq!(
        bound[1],
        ("origen_tras + 6".to_owned(), "altura_cadera / 2".to_owned()),
        "the apex above all: a place inside the cloth, stated as a formula"
    );
}

/// A press on a place along a tract still cuts a wedge, on the very piece a
/// dart has just been declared on.
///
/// The two gestures share the tool and neither shadows the other: what the
/// press landed on is what decides, and that is the snap ladder's own answer.
#[test]
fn a_press_along_a_tract_still_cuts_its_own_wedge() {
    let (mut studio, piece, nodes) = drafted();
    press_nodes(&mut studio, nodes);
    let points = studio.doc().points.len();
    let hem = ["bajo_lat_tras", "bajo_int_tras"].map(|label| {
        studio
            .doc()
            .shows_label(piece, label)
            .expect("the block names the hem")
    });

    for t in [0.3, 0.6] {
        let at = along(&studio, (hem[0], hem[1]), t);
        studio.click(at);
    }
    let mouth = resolved(&studio, hem[0]);
    let apex = studio
        .state
        .view
        .to_screen([mouth[0] - 13.0, mouth[1] - 8.0]);
    studio.click(apex);
    studio.frame(Vec::new());

    assert_eq!(studio.state.refused, None, "the cut was taken");
    assert_eq!(studio.doc().darts.len(), 2, "the declared one and this one");
    assert_eq!(
        studio.doc().points.len(),
        points + 3,
        "and cutting is what places nodes"
    );
    assert_eq!(studio.session.undo_label(), Some(PUT_ON));
}
