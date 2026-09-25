#![allow(
    clippy::float_cmp,
    reason = "a place written at the resolution of the snap is that number exactly"
)]

use eframe::egui::{Key, vec2};
use toile_engine::draft::{
    Binding, Command, Doc, EdgeAnchor, Identity, LineEdit, LineKey, LineKind, Point, PointKey,
    VertexEdit, block,
};

use super::tests::{Table, free, glass, table_of};
use super::*;
use crate::tabs::patronaje::view::View;

/// The block's front with one line of two places on the bare cloth, and the key
/// that line took.
///
/// A buttonhole, in other words: the pair of places the importer writes for one
/// and the pair the Line tool writes on bare cloth are the same thing.
fn holed() -> (Table, LineKey) {
    let mut doc = block::trouser_front();
    let piece = doc.piece_named(block::FRONT).expect("the block draws one");
    let edit = LineEdit::new(
        piece,
        LineKind::Buttonhole,
        VertexEdit::free(Point::at(3.0, 4.0)),
    )
    .to(VertexEdit::free(Point::at(3.0, 6.0)));
    let key = LineKey::new(doc.lines.issued(), 0);
    Command::AddLine {
        identity: Identity::New,
        line: Box::new(edit),
    }
    .apply(&mut doc)
    .expect("a piece takes a line of its own places");
    (table_of(doc), key)
}

/// The same with one place on the contour and one on the cloth, the way a
/// pocket mouth reads.
fn mouthed() -> (Table, LineKey) {
    let mut doc = block::trouser_front();
    let piece = doc.piece_named(block::FRONT).expect("the block draws one");
    let node = doc.shows_label(piece, "cintura_cf").expect("it is named");
    let edit = LineEdit::new(
        piece,
        LineKind::Slit,
        VertexEdit::Contour(EdgeAnchor::at_node(piece, node)),
    )
    .to(VertexEdit::free(Point::at(3.0, 6.0)));
    let key = LineKey::new(doc.lines.issued(), 0);
    Command::AddLine {
        identity: Identity::New,
        line: Box::new(edit),
    }
    .apply(&mut doc)
    .expect("a piece takes a line drawn on it");
    (table_of(doc), key)
}

/// Where the mat draws one place of the line, on the glass.
fn on_glass(table: &Table, line: LineKey, which: usize) -> eframe::egui::Pos2 {
    let drawn = table
        .drawn()
        .iter()
        .find(|it| it.line == line)
        .expect("the mat drew the line");
    View::default().to_screen(drawn.loose[which].1)
}

/// The point one place of the line sits on.
fn point_of(table: &Table, line: LineKey, which: usize) -> PointKey {
    let drawn = table
        .drawn()
        .iter()
        .find(|it| it.line == line)
        .expect("the mat drew the line");
    drawn.loose[which].0
}

/// A press on a place of the chosen line takes that place in hand, alone, and
/// opens one entry under its own name.
#[test]
fn a_press_on_a_place_of_the_chosen_line_takes_it_in_hand() {
    let (table, line) = holed();
    let ctx = table.holding(free(), Selection::Line(line));
    let at = on_glass(&table, line, 0);
    let (gesture, commands, feedback) =
        update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    assert!(commands.is_empty(), "a press is not a drag");
    assert_eq!(feedback.stack, Some(Stack::Open(place::MOVE)));
    assert_eq!(feedback.select, None, "the line stays the thing chosen");
    let Gesture::Drag(drag) = &gesture else {
        panic!("the place is in hand: {gesture:?}");
    };
    assert_eq!(drag.nodes.len(), 1, "a place brings nothing with it");
    assert_eq!(drag.nodes[0].point, point_of(&table, line, 0));
}

/// A place of a line nobody chose is not taken: the press falls through to
/// whatever the mat reads there.
#[test]
fn a_place_of_a_line_nobody_chose_is_not_taken() {
    let (table, line) = holed();
    let ctx = table.holding(free(), Selection::None);
    let at = on_glass(&table, line, 0);
    let (gesture, commands, _) = update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    assert!(
        !matches!(gesture, Gesture::Drag(_)),
        "nothing is in hand: {gesture:?}"
    );
    assert!(commands.is_empty());
}

/// The drag writes one `MovePoint` per frame, and letting go closes the one
/// entry the press opened.
#[test]
fn a_drag_of_a_place_writes_one_move_per_frame() {
    let (table, line) = holed();
    let ctx = table.holding(free(), Selection::Line(line));
    let at = on_glass(&table, line, 0);
    let point = point_of(&table, line, 0);
    let (gesture, _, _) = update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    let (gesture, commands, _) = update(
        gesture,
        Input::Move(at + vec2(glass(2.0), glass(1.0)), Mods::default()),
        &ctx,
    );
    assert_eq!(commands.len(), 1);
    let Command::MovePoint { point: moved, to } = &commands[0] else {
        panic!("a drag frame moves a point: {:?}", commands[0]);
    };
    assert_eq!(*moved, point, "the place the press took hold of");
    assert_eq!(to[0], Binding::literal(5.0), "two centimetres across");
    assert_eq!(to[1], Binding::literal(5.0), "and one down");

    let (gesture, commands, feedback) = update(gesture, Input::Up(at, Mods::default()), &ctx);
    assert_eq!(gesture, Gesture::Idle);
    assert!(commands.is_empty());
    assert_eq!(feedback.stack, Some(Stack::Close), "a literal asks nothing");
}

/// Escape mid-drag refuses the whole gesture rather than stepping back through
/// it, exactly as a node's drag does.
#[test]
fn escape_mid_drag_refuses_the_gesture() {
    let (table, line) = holed();
    let ctx = table.holding(free(), Selection::Line(line));
    let at = on_glass(&table, line, 0);
    let (gesture, _, _) = update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    let (gesture, _, _) = update(
        gesture,
        Input::Move(at + vec2(glass(2.0), 0.0), Mods::default()),
        &ctx,
    );
    let (gesture, commands, feedback) =
        update(gesture, Input::Key(Key::Escape, Mods::default()), &ctx);
    assert_eq!(gesture, Gesture::Idle);
    assert!(
        commands.is_empty(),
        "the way back is the stack, not an edit"
    );
    assert_eq!(feedback.stack, Some(Stack::Cancel));
}

/// A place written as a formula keeps it, and the release asks the same
/// question a node's drag asks — under the word the panel uses for a place.
#[test]
fn dragging_a_place_written_as_a_formula_asks_before_it_is_kept() {
    let mut doc = block::trouser_front();
    let piece = doc.piece_named(block::FRONT).expect("the block draws one");
    let x = Binding::parse("cintura / 4 - 1").expect("the source parses");
    let edit = LineEdit::new(
        piece,
        LineKind::Buttonhole,
        VertexEdit::free(Point::at(x, Binding::literal(6.0))),
    )
    .to(VertexEdit::free(Point::at(3.0, 8.0)));
    Command::AddLine {
        identity: Identity::New,
        line: Box::new(edit),
    }
    .apply(&mut doc)
    .expect("a piece takes a line of its own places");
    let line = LineKey::new(0, 0);
    let table = table_of(doc);
    let ctx = table.holding(free(), Selection::Line(line));
    let at = on_glass(&table, line, 0);

    let (gesture, _, _) = update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    let away = at + vec2(glass(1.5), 0.0);
    let (gesture, commands, _) = update(gesture, Input::Move(away, Mods::default()), &ctx);
    let Command::MovePoint { to, .. } = &commands[0] else {
        panic!("a drag frame moves a point");
    };
    assert_eq!(to[0].source(), "cintura / 4 + 0.5", "the delta is absorbed");

    let (_, _, feedback) = update(gesture, Input::Up(away, Mods::default()), &ctx);
    assert_eq!(
        feedback.stack, None,
        "the modal closes the entry, not the up"
    );
    let ask = feedback.ask.expect("a formula was rewritten");
    assert_eq!(ask.rows.len(), 1, "{ask:?}");
    assert_eq!(ask.rows[0].before, "cintura / 4 - 1");
    assert_eq!(ask.rows[0].after, "cintura / 4 + 0.5");
}

/// Only the places off the contour are taken: the one anchored to a tract is
/// not a point to drag, and the press falls through to choosing the line again.
#[test]
fn a_place_anchored_to_the_contour_is_not_one_to_drag() {
    let (table, line) = mouthed();
    let drawn = &table.drawn()[0];
    assert_eq!(drawn.places, 2);
    assert_eq!(drawn.loose.len(), 1, "one of the two is on the contour");
    let ctx = table.holding(free(), Selection::Line(line));
    // The head of the run is the anchored place: a press there takes the node
    // of the contour it sits on, which is what a press on a node always
    // does.
    let head = View::default().to_screen(drawn.run[0]);
    let (gesture, _, feedback) = update(Gesture::Idle, Input::Down(head, Mods::default()), &ctx);
    let Gesture::Drag(drag) = &gesture else {
        panic!("the node under it is in hand: {gesture:?}");
    };
    assert_ne!(drag.nodes[0].point, point_of(&table, line, 0));
    assert!(
        feedback.select.is_some(),
        "a node taken in hand is a node chosen"
    );
}

/// A document the mat can hold: the line's places are listed in run order, so a
/// press takes hold of the place it landed on and not of another of them.
#[test]
fn the_places_of_a_line_are_listed_in_the_order_the_run_names_them() {
    let (table, line) = holed();
    let drawn = table
        .drawn()
        .iter()
        .find(|it| it.line == line)
        .expect("the mat drew it");
    assert_eq!(drawn.loose.len(), 2);
    assert_eq!(drawn.loose[0].1, [3.0, 4.0]);
    assert_eq!(drawn.loose[1].1, [3.0, 6.0]);
    assert_eq!(drawn.run, vec![[3.0, 4.0], [3.0, 6.0]]);
}

/// A line whose places resolve nowhere offers none to take hold of, rather than
/// offering one at the origin.
#[test]
fn a_line_whose_place_resolves_nowhere_offers_nothing_to_take() {
    let mut doc: Doc = block::trouser_front();
    let piece = doc.piece_named(block::FRONT).expect("the block draws one");
    let edit = LineEdit::new(
        piece,
        LineKind::Reference,
        VertexEdit::free(Point::at(
            Binding::parse("no_existe").expect("the source parses"),
            Binding::literal(2.0),
        )),
    )
    .to(VertexEdit::free(Point::at(3.0, 8.0)));
    Command::AddLine {
        identity: Identity::New,
        line: Box::new(edit),
    }
    .apply(&mut doc)
    .expect("a binding is checked when it resolves, not when it is written");
    let table = table_of(doc);
    let drawn = &table.drawn()[0];
    assert!(drawn.run.is_empty(), "nothing is drawn for it");
    assert_eq!(drawn.loose.len(), 1, "and only the place that resolves");
}
