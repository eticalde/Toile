#![allow(
    clippy::float_cmp,
    reason = "a place written at the resolution of the snap is that number exactly"
)]

use eframe::egui::{Event, Key, Modifiers, Pos2};
use toile_engine::draft::{
    Command, Doc, EdgeAnchor, Identity, LineKey, LineVertex, Notch, NotchKey, PieceKey, PointKey,
    block,
};

use super::super::gesture::Gesture;
use super::super::state::{Scope, Selection, Tool};
use super::button;
use super::studio::Studio;

/// A place well inside the paper of the front and clear of every tract: three
/// centimetres in from the corner the waist and the centre front meet at.
const ON_THE_CLOTH: [f64; 2] = [3.0, 3.0];

/// Where the pointer has to go to reach that place once it has moved two
/// centimetres across and one down.
const MOVED_TO: [f64; 2] = [5.0, 4.0];

/// The front alone on the mat, framed, with every tool in hand.
fn detail(doc: Doc) -> (Studio, PieceKey) {
    let mut studio = Studio::new(doc);
    let piece = studio.doc().piece_keys()[0];
    studio.state.scope = Scope::Piece;
    studio.state.active = Some(piece);
    studio.frame(Vec::new());
    (studio, piece)
}

/// Where a place in the pattern's own centimetres lands on the glass right now.
fn at_cm(studio: &Studio, cm: [f64; 2]) -> Pos2 {
    studio.state.view.to_screen(cm)
}

/// The pointer pressing at one place, dragging to another and letting go.
fn drag(studio: &mut Studio, from: Pos2, to: Pos2) {
    studio.frame(vec![Event::PointerMoved(from)]);
    studio.frame(vec![button(from, true)]);
    studio.frame(vec![Event::PointerMoved(to)]);
    studio.frame(vec![button(to, false)]);
}

/// The one line the product holds, and the places of its own it runs through.
fn only(studio: &Studio) -> (LineKey, Vec<PointKey>) {
    let doc = studio.doc();
    assert_eq!(doc.lines.len(), 1, "one line, no more");
    let (key, held) = doc.lines.iter().next().expect("the arena holds one");
    (key, held.vertices().filter_map(LineVertex::point).collect())
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

/// A buttonhole traced on the bare cloth of the front: two presses and Enter.
fn holed() -> Studio {
    let (mut studio, _) = detail(block::trouser_front());
    studio.key(Key::L, Modifiers::NONE);
    assert_eq!(studio.state.tool, Tool::Trace, "L takes the line tool");
    studio.click(at_cm(&studio, ON_THE_CLOTH));
    assert!(matches!(studio.state.gesture, Gesture::Tracing(_)));
    studio.click(at_cm(&studio, [ON_THE_CLOTH[0], ON_THE_CLOTH[1] + 2.0]));
    studio.key(Key::Enter, Modifiers::NONE);
    studio.frame(Vec::new());
    studio
}

/// Two presses on the bare cloth and Enter draw a line of two places of its
/// own, as one entry, and the places land where the presses did.
///
/// That is the mark the contour has nowhere to put: a buttonhole, a button, a
/// belt-loop mark. Undo takes the line and both of its points away together.
#[test]
fn the_line_tool_draws_a_mark_on_the_bare_cloth_in_one_entry() {
    let studio = holed();
    let (key, places) = only(&studio);
    assert_eq!(places.len(), 2, "both places are the line's own");
    assert_eq!(resolved(&studio, places[0]), ON_THE_CLOTH);
    assert_eq!(
        resolved(&studio, places[1]),
        [ON_THE_CLOTH[0], ON_THE_CLOTH[1] + 2.0]
    );
    assert_eq!(studio.session.undo_label(), Some("trazar línea"));
    assert_eq!(studio.state.gesture, Gesture::Idle);
    assert_eq!(studio.state.selection, Selection::Line(key), "and chosen");

    let mut studio = studio;
    let points = studio.doc().points.len();
    studio.session.undo().expect("the step back");
    studio.frame(Vec::new());
    assert_eq!(studio.doc().lines.len(), 0, "the cloth is bare again");
    assert_eq!(
        studio.doc().points.len(),
        points - 2,
        "and the two places went with the line"
    );
}

/// A place of the chosen line is dragged by hand: one entry, the point moves to
/// where the pointer let go, and undo puts it back.
#[test]
fn a_place_of_the_chosen_line_is_dragged_by_hand() {
    let mut studio = holed();
    let (line, places) = only(&studio);
    studio.key(Key::V, Modifiers::NONE);
    studio.frame(Vec::new());
    assert_eq!(studio.state.selection, Selection::Line(line));

    let from = at_cm(&studio, ON_THE_CLOTH);
    let to = at_cm(&studio, MOVED_TO);
    drag(&mut studio, from, to);
    assert_eq!(resolved(&studio, places[0]), MOVED_TO);
    assert_eq!(
        resolved(&studio, places[1]),
        [ON_THE_CLOTH[0], ON_THE_CLOTH[1] + 2.0],
        "the other place stayed where it was"
    );
    assert_eq!(studio.session.undo_label(), Some("mover lugar"));
    assert_eq!(
        studio.state.selection,
        Selection::Line(line),
        "the line is still what is chosen"
    );

    studio.session.undo().expect("the step back");
    studio.frame(Vec::new());
    assert_eq!(
        resolved(&studio, places[0]),
        ON_THE_CLOTH,
        "one entry, one step"
    );
}

/// A mark drawn for one body is still that mark on another: it resolves where
/// the hand put it, and when a measurement changes it travels with the cloth
/// around it rather than staying behind.
///
/// The place is written from the node nearest it — here the side of the waist,
/// whose x is `cintura / 4 + 1` — so a waist four centimetres wider moves both
/// the node and the mark by one.
#[test]
fn a_mark_drawn_on_the_cloth_resolves_where_it_was_drawn_and_travels_with_it() {
    let (mut studio, piece) = detail(block::trouser_front());
    let side = studio
        .doc()
        .shows_label(piece, "cintura_lat")
        .expect("the block names it");
    let waist = resolved(&studio, side);
    let first = [waist[0] - 2.0, waist[1] + 2.0];
    let second = [first[0], first[1] + 2.0];
    studio.key(Key::L, Modifiers::NONE);
    studio.click(at_cm(&studio, first));
    studio.click(at_cm(&studio, second));
    studio.key(Key::Enter, Modifiers::NONE);
    studio.frame(Vec::new());

    let (_, places) = only(&studio);
    assert_eq!(resolved(&studio, places[0]), first, "where it was drawn");
    let before = resolved(&studio, side);

    let mannequin = studio.doc().resolve_with;
    studio
        .session
        .edit(Command::SetMeasure {
            mannequin,
            name: "cintura".to_owned(),
            to: 88.0,
        })
        .expect("the body carries a waist");
    studio.frame(Vec::new());
    let node = resolved(&studio, side);
    let mark = resolved(&studio, places[0]);
    assert_eq!(node[0] - before[0], 1.0, "a waist of 88 moves the node one");
    assert_eq!(
        mark[0] - first[0],
        node[0] - before[0],
        "and the mark goes with it"
    );
    assert_eq!(mark[1], first[1], "nothing moved it down");
}

/// The block's front with one notch cut half way along the tract leaving the
/// centre front of the waist, which runs straight across the piece.
fn marked() -> (Studio, NotchKey, PointKey) {
    marked_at(0.5)
}

/// The same, with the mark cut at whatever fraction of that tract.
fn marked_at(t: f64) -> (Studio, NotchKey, PointKey) {
    let mut doc = block::trouser_front();
    let piece = doc.piece_named(block::FRONT).expect("the block draws one");
    let from = doc
        .shows_label(piece, "cintura_cf")
        .expect("the block names it");
    let applied = Command::AddNotch {
        identity: Identity::New,
        notch: Notch::lone(EdgeAnchor { piece, from, t }),
        mate: None,
    }
    .apply(&mut doc)
    .expect("a fraction of a tract is a place on the contour");
    let Command::RemoveNotch { notch } = applied.inverse else {
        panic!("the inverse takes the mark off the contour");
    };
    let (studio, _) = detail(doc);
    (studio, notch, from)
}

/// A notch is dragged along its own tract: one entry, the mark slides, and it
/// never leaves the tract it was cut into.
#[test]
fn a_notch_is_dragged_along_its_tract_in_one_entry() {
    let (mut studio, notch, from) = marked();
    let at = |studio: &Studio| {
        studio
            .doc()
            .notches
            .get(notch)
            .expect("the mark is live")
            .at
    };
    // The waist runs straight from the centre front to the side, so half its
    // arc length is half way between the two nodes, which is where the mat
    // draws the mark.
    let piece = studio.state.active.expect("a piece is in front");
    let side = studio
        .doc()
        .shows_label(piece, "cintura_lat")
        .expect("the block names it");
    let ends = (resolved(&studio, from), resolved(&studio, side));
    let start = [0, 1].map(|k| f64::midpoint(ends.0[k], ends.1[k]));
    let (grabbed, along) = (
        at_cm(&studio, start),
        at_cm(&studio, [start[0] + 5.0, start[1] + 1.0]),
    );
    drag(&mut studio, grabbed, along);
    let moved = at(&studio);
    assert_eq!(moved.from, from, "the mark stayed on its tract");
    assert!(moved.t > 0.5, "and slid along it: {}", moved.t);
    assert_eq!(studio.session.undo_label(), Some("mover piquete"));

    studio.session.undo().expect("the step back");
    studio.frame(Vec::new());
    assert_eq!(at(&studio).t, 0.5, "one drag, one step back");
}

/// A mark cut at a node is dragged with its own tract chosen, and the cut line
/// does not move.
///
/// The case every imported notch is in: in Seamly a notch is written on an
/// outline node, so its fraction is zero and the mat draws the cut under the
/// node's own dot. One click on the tract says which of the two the next press
/// means; without it the press takes the node, and dragging a node is an edit
/// to the cut line rather than to the mark.
#[test]
fn a_mark_cut_at_a_node_slides_and_leaves_the_node_where_it_was() {
    let (mut studio, notch, from) = marked_at(0.0);
    let piece = studio.state.active.expect("a piece is in front");
    let side = studio
        .doc()
        .shows_label(piece, "cintura_lat")
        .expect("the block names it");
    let ends = (resolved(&studio, from), resolved(&studio, side));
    let middle = [0, 1].map(|k| f64::midpoint(ends.0[k], ends.1[k]));
    studio.click(at_cm(&studio, middle));
    assert_eq!(studio.state.selection, Selection::Edge(from));

    let along = [ends.0[0] + 3.0, ends.0[1]];
    let (grabbed, over) = (at_cm(&studio, ends.0), at_cm(&studio, along));
    drag(&mut studio, grabbed, over);
    let moved = studio
        .doc()
        .notches
        .get(notch)
        .expect("the mark is live")
        .at;
    assert_eq!(moved.from, from, "the mark stayed on its tract");
    assert!(moved.t > 0.0, "and slid along it: {}", moved.t);
    assert_eq!(resolved(&studio, from), ends.0, "the node did not move");
    assert_eq!(studio.session.undo_label(), Some("mover piquete"));
}
