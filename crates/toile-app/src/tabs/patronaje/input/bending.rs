use std::collections::BTreeSet;

use eframe::egui::vec2;
use toile_engine::draft::PointKey;

use super::super::snap::SnapKind;
use super::square::{HANDLES, SQUARE, alt, bent, glass, moved_to, on_glass, play};
use super::*;

#[test]
fn dragging_a_handle_does_not_change_the_point_count() {
    let table = bent();
    let (handle, _) = table.pair();
    let ctx = table.holding(Selection::point(handle), Tool::Select);
    let at = on_glass(HANDLES[0]);
    let before = table.draft.flat_cm(table.piece).len();

    let (gesture, _, feedback) = update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    assert_eq!(feedback.select, Some(Selection::point(handle)));
    let (_, commands, _) = update(
        gesture,
        Input::Move(at + vec2(glass(1.5), glass(-2.0)), Mods::default()),
        &ctx,
    );
    let after = table.after(&commands);
    assert!(after.defects(table.piece).is_empty());
    assert_eq!(
        after.flat_cm(table.piece).len(),
        before,
        "the sample count decides the point count, and the drag did not touch it"
    );
    assert_eq!(after.points_cm(table.piece).len(), table.nodes.len());
}

#[test]
fn a_handle_carries_its_mate_the_other_way() {
    let table = bent();
    let (handle, mate) = table.pair();
    let ctx = table.holding(Selection::point(handle), Tool::Select);
    let at = on_glass(HANDLES[0]);
    let (gesture, _, _) = update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    let (_, commands, _) = update(
        gesture,
        Input::Move(at + vec2(glass(2.0), 0.0), Mods::default()),
        &ctx,
    );
    assert_eq!(commands.len(), 2, "the handle and its mate: {commands:?}");
    let after = table.after(&commands);
    let (pulled, to) = moved_to(&commands[0], &after);
    let (other, back) = moved_to(&commands[1], &after);
    assert_eq!(pulled, handle);
    assert_eq!(other, mate);
    assert!((to[0] - (HANDLES[0][0] + 2.0)).abs() < 1.0e-9, "{to:?}");
    assert!((back[0] - (HANDLES[3][0] - 2.0)).abs() < 1.0e-9, "{back:?}");
}

#[test]
fn alt_breaks_the_tangent_and_releasing_does_not_restore_it() {
    let table = bent();
    let (handle, mate) = table.pair();
    let ctx = table.holding(Selection::point(handle), Tool::Select);
    let at = on_glass(HANDLES[0]);
    let (gesture, _, _) = update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    let mut played = table.draft.clone();

    // A frame with the pairing whole: the mate takes the delta reversed.
    let (gesture, commands, _) = update(
        gesture,
        Input::Move(at + vec2(glass(2.0), 0.0), Mods::default()),
        &ctx,
    );
    assert_eq!(commands.len(), 2);
    play(&mut played, &commands);
    assert_eq!(
        played.resolved(mate),
        Some([HANDLES[3][0] - 2.0, HANDLES[3][1]])
    );

    // Alt: the mate is let go, where the last frame left it.
    let (gesture, commands, _) = update(
        gesture,
        Input::Move(at + vec2(glass(3.0), 0.0), alt()),
        &ctx,
    );
    assert_eq!(commands.len(), 1, "the mate is let go: {commands:?}");
    assert_eq!(moved_to(&commands[0], &played).0, handle);
    play(&mut played, &commands);
    let broken = played.resolved(mate);
    assert_eq!(broken, Some([HANDLES[3][0] - 2.0, HANDLES[3][1]]));

    // The key comes back up, and the break holds: the mate does not jump back
    // to where symmetry about the node would have kept it.
    let (gesture, commands, _) = update(
        gesture,
        Input::Move(at + vec2(glass(4.0), 0.0), Mods::default()),
        &ctx,
    );
    assert_eq!(commands.len(), 1, "the break latches: {commands:?}");
    play(&mut played, &commands);
    assert_eq!(played.resolved(mate), broken);

    let (gesture, commands, feedback) = update(
        gesture,
        Input::Up(at + vec2(glass(4.0), 0.0), Mods::default()),
        &ctx,
    );
    assert_eq!(gesture, Gesture::Idle);
    assert!(commands.is_empty(), "letting go writes nothing");
    assert_eq!(
        feedback.stack,
        Some(Stack::Close),
        "one entry, and it closes"
    );
    assert_eq!(played.resolved(mate), broken);
}

#[test]
fn dragging_a_node_carries_its_handles_in_one_gesture() {
    let table = bent();
    let corner = table.nodes[0].0;
    let (out, into) = table.pair();
    let ctx = table.holding(Selection::None, Tool::Select);
    let at = on_glass(SQUARE[0]);

    let (gesture, commands, feedback) =
        update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    assert!(commands.is_empty(), "a press edits nothing");
    assert_eq!(feedback.stack, Some(Stack::Open(MOVE)), "one entry opens");

    let (gesture, commands, feedback) = update(
        gesture,
        Input::Move(at + vec2(glass(1.0), glass(1.0)), Mods::default()),
        &ctx,
    );
    assert_eq!(commands.len(), 3, "the node and its two handles");
    assert!(feedback.stack.is_none(), "and no second entry");
    let after = table.after(&commands);
    for (command, was) in commands.iter().zip([SQUARE[0], HANDLES[0], HANDLES[3]]) {
        let (_, to) = moved_to(command, &after);
        assert!(
            (to[0] - (was[0] + 1.0)).abs() < 1.0e-9 && (to[1] - (was[1] + 1.0)).abs() < 1.0e-9,
            "every point in hand takes the same delta: {to:?} from {was:?}"
        );
    }
    let held: Vec<PointKey> = commands
        .iter()
        .map(|command| moved_to(command, &after).0)
        .collect();
    assert_eq!(held, vec![corner, out, into]);

    let (_, commands, feedback) = update(gesture, Input::Up(at, Mods::default()), &ctx);
    assert!(commands.is_empty());
    assert_eq!(feedback.stack, Some(Stack::Close), "and one entry closes");
}

/// A drag measures every frame from where it took hold, never from the frame
/// before. A candidate the gesture is itself carrying would break that: the
/// handles of a chosen node are in hand and on show at once, so without the
/// rule the pointer could latch a node onto its own tangent, wherever the
/// last frame happened to leave it.
#[test]
fn a_drag_does_not_catch_a_point_it_is_carrying() {
    let table = bent();
    let corner = table.nodes[0].0;
    let (out, into) = table.pair();
    let ctx = table.snapping(Selection::point(corner), Tool::Select);
    let at = on_glass(SQUARE[0]);
    let (gesture, _, _) = update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);

    // Straight at the handle hanging off the very corner in hand.
    let (_, _, feedback) = update(
        gesture,
        Input::Move(on_glass(HANDLES[0]), Mods::default()),
        &ctx,
    );
    let caught = feedback.snapped.expect("a drag frame says what it caught");
    for carried in [
        SnapKind::Handle(out),
        SnapKind::Handle(into),
        SnapKind::Node(corner),
    ] {
        assert_ne!(caught.kind, Some(carried), "{caught:?}");
    }
}

#[test]
fn a_handle_the_selection_already_holds_is_not_taken_twice() {
    let table = bent();
    let corner = table.nodes[0].0;
    let (out, _) = table.pair();
    let both = Selection::Points(BTreeSet::from([corner, out]));
    let ctx = table.holding(both, Tool::Select);
    let at = on_glass(SQUARE[0]);
    let (gesture, _, _) = update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    let (_, commands, _) = update(
        gesture,
        Input::Move(at + vec2(glass(1.0), 0.0), Mods::default()),
        &ctx,
    );
    let after = table.after(&commands);
    let mut held: Vec<PointKey> = commands
        .iter()
        .map(|command| moved_to(command, &after).0)
        .collect();
    let taken = held.len();
    held.sort_by_key(|key| (key.index(), key.generation()));
    held.dedup();
    assert_eq!(held.len(), taken, "one move per point, never two");
}
