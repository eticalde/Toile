#![allow(
    clippy::float_cmp,
    reason = "a vertex placed with the snap off is stored exactly as clicked"
)]

use toile_engine::draft::{Identity, PieceKey, SegmentEdit, Winding};

use super::super::snap::SnapKind;
use super::draw::DRAW;
use super::square::{bent, on_glass};
use super::*;

/// The corners of the piece the tests draw, clear of the square already on
/// the table, clockwise on the page.
const CORNERS: [[f64; 2]; 4] = [[20.0, 0.0], [30.0, 0.0], [30.0, 8.0], [20.0, 8.0]];

/// Feeds one press per corner and hands back the gesture it leaves.
fn placed(ctx: &EditContext<'_>, corners: &[[f64; 2]]) -> Gesture {
    let mut gesture = Gesture::Idle;
    for &at in corners {
        let (next, commands, feedback) =
            update(gesture, Input::Down(on_glass(at), Mods::default()), ctx);
        gesture = next;
        assert!(commands.is_empty(), "no command until the contour closes");
        assert_eq!(feedback.stack, None);
    }
    gesture
}

/// The vertices a drawing holds so far.
fn pending(gesture: &Gesture) -> &[[f64; 2]] {
    let Gesture::Drawing { pending, .. } = gesture else {
        panic!("the tool is drawing: {gesture:?}");
    };
    pending
}

#[test]
fn the_whole_drawing_gesture_makes_one_piece_in_one_entry() {
    let table = bent();
    let ctx = table.holding(Selection::None, Tool::Line);
    let gesture = placed(&ctx, &CORNERS);
    assert_eq!(pending(&gesture), &CORNERS);

    // Coming back to the first vertex closes the contour.
    let (gesture, commands, feedback) = update(
        gesture,
        Input::Down(on_glass(CORNERS[0]), Mods::default()),
        &ctx,
    );
    assert_eq!(gesture, Gesture::Idle);
    assert_eq!(feedback.stack, Some(Stack::Once(DRAW)));
    assert_eq!(commands.len(), 1 + CORNERS.len());

    let Command::AddPiece {
        identity: Identity::New,
        piece,
    } = &commands[0]
    else {
        panic!("the piece goes first: {:?}", commands[0]);
    };
    assert_eq!(piece.name, "Pieza 1");
    assert_eq!(piece.winding, Winding::Cw);
    assert!(
        piece.contour.is_empty(),
        "the vertices follow as insertions"
    );

    // Every insertion lands at the head of the piece the arena is about to
    // issue, in reverse click order, so the contour ends up as drawn.
    let predicted = PieceKey::new(table.draft.doc().pieces.issued(), 0);
    for (index, command) in commands[1..].iter().enumerate() {
        let Command::InsertNode {
            piece,
            after: None,
            identity: Identity::New,
            value,
            segment: SegmentEdit::Line,
            samples: 1,
        } = command
        else {
            panic!("a straight vertex at the head: {command:?}");
        };
        assert_eq!(*piece, predicted);
        let at = [&value.x, &value.y]
            .map(|axis| axis.eval(table.draft.env()).expect("a literal resolves"));
        assert_eq!(at, CORNERS[CORNERS.len() - 1 - index]);
    }

    // Applied under one entry, the piece is on the table as clicked — and one
    // undo takes the whole of it back, points and all.
    let mut draft = table.draft.clone();
    let points = draft.doc().points.len();
    draft.begin_gesture(DRAW);
    for command in &commands {
        draft.edit(command.clone()).expect("the drawing applies");
    }
    draft.end_gesture();
    let key = draft
        .doc()
        .piece_named("Pieza 1")
        .expect("the drawn piece is on the table");
    assert_eq!(key, predicted);
    let order: Vec<[f64; 2]> = draft.points_cm(key).iter().map(|&(_, at)| at).collect();
    assert_eq!(order, CORNERS);
    assert!(draft.defects(key).is_empty(), "{:?}", draft.defects(key));

    draft.undo().expect("one gesture, one entry");
    assert!(draft.doc().piece_named("Pieza 1").is_none());
    assert_eq!(draft.doc().points.len(), points);
}

#[test]
fn enter_closes_the_contour_and_a_triangle_is_enough() {
    let table = bent();
    let ctx = table.holding(Selection::None, Tool::Line);
    let gesture = placed(&ctx, &CORNERS[..3]);
    let (gesture, commands, feedback) =
        update(gesture, Input::Key(Key::Enter, Mods::default()), &ctx);
    assert_eq!(gesture, Gesture::Idle);
    assert_eq!(feedback.stack, Some(Stack::Once(DRAW)));
    assert_eq!(commands.len(), 4);
}

#[test]
fn a_contour_of_two_vertices_does_not_close() {
    let table = bent();
    let ctx = table.holding(Selection::None, Tool::Line);
    let gesture = placed(&ctx, &CORNERS[..2]);
    let (gesture, commands, _) = update(gesture, Input::Key(Key::Enter, Mods::default()), &ctx);
    assert!(commands.is_empty(), "two vertices are a line, not a piece");
    assert_eq!(pending(&gesture).len(), 2);
}

#[test]
fn escape_walks_away_from_the_drawing_commandless() {
    let table = bent();
    let ctx = table.holding(Selection::None, Tool::Line);
    let gesture = placed(&ctx, &CORNERS[..3]);
    let (gesture, commands, feedback) =
        update(gesture, Input::Key(Key::Escape, Mods::default()), &ctx);
    assert_eq!(gesture, Gesture::Idle);
    assert!(commands.is_empty());
    // Not even a Cancel: nothing was opened, so there is nothing to unwind.
    assert_eq!(feedback.stack, None);
}

#[test]
fn backspace_takes_the_last_vertex_back() {
    let table = bent();
    let ctx = table.holding(Selection::None, Tool::Line);
    let gesture = placed(&ctx, &CORNERS[..3]);
    let (gesture, commands, _) = update(gesture, Input::Key(Key::Backspace, Mods::default()), &ctx);
    assert!(commands.is_empty());
    assert_eq!(pending(&gesture), &CORNERS[..2]);
}

#[test]
fn the_rubber_line_follows_the_pointer() {
    let table = bent();
    let ctx = table.holding(Selection::None, Tool::Line);
    let gesture = placed(&ctx, &CORNERS[..1]);
    let (gesture, _, feedback) = update(
        gesture,
        Input::Move(on_glass([26.0, 3.0]), Mods::default()),
        &ctx,
    );
    let Gesture::Drawing { rubber, .. } = gesture else {
        panic!("a move keeps the drawing: {gesture:?}");
    };
    assert_eq!(rubber, [26.0, 3.0]);
    assert!(feedback.snapped.is_some(), "the candidate is always shown");
}

#[test]
fn the_line_tool_draws_instead_of_grabbing() {
    let table = bent();
    // The snap is live: a press on an existing node is a vertex caught by
    // that node, not a drag taking it in hand.
    let ctx = table.snapping(Selection::None, Tool::Line);
    let at = table.nodes[2].1;
    let (gesture, commands, feedback) = update(
        Gesture::Idle,
        Input::Down(on_glass(at), Mods::default()),
        &ctx,
    );
    assert!(commands.is_empty());
    assert!(matches!(gesture, Gesture::Drawing { .. }), "{gesture:?}");
    assert_eq!(pending(&gesture), &[at]);
    let caught = feedback.snapped.expect("the press reports what caught it");
    assert_eq!(caught.kind, Some(SnapKind::Node(table.nodes[2].0)));
}
