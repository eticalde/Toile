use toile_engine::draft::{Command, Identity, SegmentEdit};

use super::node::{INSERT, REMOVE};
use super::square::{SAMPLES, bent, on_glass};
use super::*;

/// A binding of the square's document, resolved to centimetres.
fn resolved(table: &super::square::Table, binding: &toile_engine::draft::Binding) -> f64 {
    binding
        .eval(table.draft.env())
        .expect("the binding resolves")
}

#[test]
fn the_point_tool_drops_a_node_on_a_straight_tract_in_one_entry() {
    let table = bent();
    let ctx = table.holding(Selection::None, Tool::Point);
    // The middle of the square's right side, which is a straight tract.
    let (gesture, commands, feedback) = update(
        Gesture::Idle,
        Input::Down(on_glass([10.0, 5.0]), Mods::default()),
        &ctx,
    );
    assert_eq!(
        gesture,
        Gesture::Idle,
        "an insertion is a click, not a drag"
    );
    assert_eq!(feedback.stack, Some(Stack::Once(INSERT)));

    let [
        Command::InsertNode {
            piece,
            after: Some(after),
            identity: Identity::New,
            value,
            segment: SegmentEdit::Line,
            samples: 1,
        },
    ] = commands.as_slice()
    else {
        panic!("one straight insertion: {commands:?}");
    };
    assert_eq!(*piece, table.piece);
    assert_eq!(*after, table.nodes[1].0, "the tract leaving the top right");
    // On the line itself, not pulled to the grid: the glass round-trips
    // through f32, so exactness here is a hair, not a bit pattern.
    assert!((resolved(&table, &value.x) - 10.0).abs() < 1.0e-3);
    assert!((resolved(&table, &value.y) - 5.0).abs() < 1.0e-3);

    let after = table.after(&commands);
    assert_eq!(after.points_cm(table.piece).len(), table.nodes.len() + 1);
    assert!(after.defects(table.piece).is_empty());
}

#[test]
fn the_point_tool_cuts_a_bent_tract_without_moving_the_line() {
    let table = bent();
    let ctx = table.holding(Selection::None, Tool::Point);
    // Where the square's top tract bows to at its middle: on the curve, well
    // away from the chord between its corners.
    let middle = [5.0, -0.75];
    let (_, commands, feedback) = update(
        Gesture::Idle,
        Input::Down(on_glass(middle), Mods::default()),
        &ctx,
    );
    assert_eq!(feedback.stack, Some(Stack::Once(INSERT)));

    let [
        Command::SetSegment {
            piece,
            node,
            to: SegmentEdit::Cubic(_),
        },
        Command::InsertNode {
            after: Some(after),
            value,
            segment: SegmentEdit::Cubic(_),
            samples,
            ..
        },
    ] = commands.as_slice()
    else {
        panic!("a cut rewrites the tract and opens the second half: {commands:?}");
    };
    assert_eq!(*piece, table.piece);
    assert_eq!(*node, table.nodes[0].0);
    assert_eq!(*after, table.nodes[0].0);
    assert_eq!(
        *samples, SAMPLES,
        "a cut is not the place to coarsen a curve"
    );
    assert!((resolved(&table, &value.x) - middle[0]).abs() < 1.0e-3);
    assert!((resolved(&table, &value.y) - middle[1]).abs() < 1.0e-3);

    let cut = table.after(&commands);
    assert_eq!(cut.points_cm(table.piece).len(), table.nodes.len() + 1);
    assert!(cut.defects(table.piece).is_empty());
}

#[test]
fn delete_takes_the_chosen_node_out_by_key() {
    let table = bent();
    let corner = table.nodes[2].0;
    let ctx = table.holding(Selection::point(corner), Tool::Select);
    let (gesture, commands, feedback) = update(
        Gesture::Idle,
        Input::Key(Key::Delete, Mods::default()),
        &ctx,
    );
    assert_eq!(gesture, Gesture::Idle);
    assert_eq!(feedback.stack, Some(Stack::Once(REMOVE)));
    assert_eq!(
        commands,
        vec![Command::RemoveNode {
            piece: table.piece,
            node: corner,
        }]
    );
    assert_eq!(
        feedback.select,
        Some(Selection::None),
        "what was deleted cannot stay chosen"
    );
    let after = table.after(&commands);
    assert_eq!(after.points_cm(table.piece).len(), table.nodes.len() - 1);
}

#[test]
fn a_chosen_handle_is_not_a_node_and_delete_leaves_it() {
    let table = bent();
    let (handle, _) = table.pair();
    let ctx = table.holding(Selection::point(handle), Tool::Select);
    let (gesture, commands, feedback) = update(
        Gesture::Idle,
        Input::Key(Key::Backspace, Mods::default()),
        &ctx,
    );
    assert_eq!(gesture, Gesture::Idle);
    assert!(
        commands.is_empty(),
        "the way to take a handle away is to straighten its tract"
    );
    assert_eq!(feedback.stack, None);
    assert_eq!(feedback.select, None);
}
