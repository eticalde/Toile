use toile_engine::draft::{Command, SegmentEdit};

use super::super::curve;
use super::square::{SAMPLES, bent, on_glass};
use super::*;

#[test]
fn the_curve_tool_bends_a_straight_tract_in_one_entry() {
    let table = bent();
    // The tract along the bottom of the square, which is straight.
    let node = table.nodes[2].0;
    let ctx = table.holding(Selection::None, Tool::Curve);
    let at = on_glass([5.0, 10.0]);

    let (gesture, commands, feedback) =
        update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    assert_eq!(gesture, Gesture::Idle, "bending is a click, not a drag");
    assert_eq!(feedback.select, Some(Selection::Edge(node)));
    assert!(
        matches!(feedback.stack, Some(Stack::Once(_))),
        "{feedback:?}"
    );
    assert_eq!(commands.len(), 2);
    assert_eq!(
        commands[0],
        Command::SetSamples {
            piece: table.piece,
            node,
            to: curve::SAMPLES
        }
    );
    assert!(matches!(
        &commands[1],
        Command::SetSegment {
            node: at,
            to: SegmentEdit::Cubic(_),
            ..
        } if *at == node
    ));

    // The handles land on the thirds of the chord, so the tract keeps the
    // shape it had: what the click buys is the ability to bend it.
    let after = table.after(&commands);
    assert!(after.defects(table.piece).is_empty());
    let length = after.run_length_cm(table.piece, node, table.nodes[3].0);
    assert!((length - 10.0).abs() < 1.0e-6, "{length}");
}

#[test]
fn the_curve_tool_leaves_a_tract_that_already_bends_alone() {
    let table = bent();
    let ctx = table.holding(Selection::None, Tool::Curve);
    let bent_at = table.tracts[0].line[SAMPLES as usize / 2];
    let (_, commands, feedback) = update(
        Gesture::Idle,
        Input::Down(on_glass(bent_at), Mods::default()),
        &ctx,
    );
    assert!(commands.is_empty(), "a second click does not redraw it");
    assert_eq!(feedback.select, Some(Selection::Edge(table.nodes[0].0)));
}

#[test]
fn the_tool_keys_put_a_tool_in_hand_and_choose_nothing() {
    let table = bent();
    let ctx = table.holding(Selection::None, Tool::Select);
    let (_, _, feedback) = update(Gesture::Idle, Input::Key(Key::C, Mods::default()), &ctx);
    assert_eq!(feedback.tool, Some(Tool::Curve));
    assert_eq!(feedback.select, None);
    let (_, _, feedback) = update(Gesture::Idle, Input::Key(Key::V, Mods::default()), &ctx);
    assert_eq!(feedback.tool, Some(Tool::Select));
    let paste = Mods {
        command: true,
        ..Mods::default()
    };
    let (_, _, feedback) = update(Gesture::Idle, Input::Key(Key::C, paste), &ctx);
    assert_eq!(feedback.tool, None, "the platform modifier is not a tool");
}
