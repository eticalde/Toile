use eframe::egui::vec2;

use super::*;

/// Two ten centimetre squares side by side, twenty apart, both drawn the same
/// way round: along the top, down the right, back along the bottom, up the
/// left.
fn squares() -> Vec<Spread> {
    [0.0, 30.0]
        .into_iter()
        .enumerate()
        .map(|(index, x)| {
            let corners = [[x, 0.0], [x + 10.0, 0.0], [x + 10.0, 10.0], [x, 10.0]];
            let base = index as u32 * 4;
            let tracts = (0..4)
                .map(|k| Tract {
                    node: PointKey::new(base + k as u32, 0),
                    to: PointKey::new(base + (k as u32 + 1) % 4, 0),
                    line: vec![corners[k], corners[(k + 1) % 4]],
                })
                .collect();
            Spread {
                piece: PieceKey::new(index as u32, 0),
                tracts,
            }
        })
        .collect()
}

/// The tract of one square by its place in the contour: 0 the top, 1 the right
/// side, 2 the bottom, 3 the left side.
fn pick(spread: &[Spread], piece: usize, tract: usize) -> Pick {
    let held = &spread[piece].tracts[tract];
    Pick {
        piece: spread[piece].piece,
        from: held.node,
        to: held.to,
    }
}

fn down(cm: [f64; 2]) -> Input {
    Input::Down(View::default().to_screen(cm), Mods::default())
}

fn key(key: Key) -> Input {
    Input::Key(key, Mods::default())
}

/// The right side of the first square, and the left side of the second.
const RIGHT_OF_FIRST: [f64; 2] = [10.0, 5.0];
const LEFT_OF_SECOND: [f64; 2] = [30.0, 5.0];

#[test]
fn two_presses_on_two_tracts_sew_them_in_one_entry() {
    let (spread, view) = (squares(), View::default());
    let (gesture, commands, said) = update(Gesture::Idle, down(RIGHT_OF_FIRST), &spread, view);
    let first = pick(&spread, 0, 1);
    assert_eq!(gesture, Gesture::Sewing(Sewing { first, pan: None }));
    assert!(commands.is_empty(), "one side is not a seam yet");
    assert_eq!(said.stack, None, "and nothing has been opened for it");
    assert_eq!(said.chosen, Some(spread[0].piece));

    let (gesture, commands, said) = update(gesture, down(LEFT_OF_SECOND), &spread, view);
    assert_eq!(gesture, Gesture::Idle);
    assert_eq!(said.stack, Some(Stack::Once(SEW)));
    let second = pick(&spread, 1, 3);
    // The right side runs down and the left side runs up, and the two squares
    // lie the same way up: top meets top, so the sides run opposed.
    let seam = Seam::plain(first.range(), second.range(), SeamOrientation::Opposed);
    let sewn = Command::AddSeam {
        identity: Identity::New,
        seam,
    };
    assert_eq!(commands, vec![sewn]);
}

#[test]
fn two_sides_that_run_the_same_way_down_the_mat_are_sewn_aligned() {
    let spread = squares();
    let right = |piece: usize| &spread[piece].tracts[1];
    assert_eq!(facing(right(0), right(1)), SeamOrientation::Aligned);
    let left = &spread[1].tracts[3];
    assert_eq!(facing(right(0), left), SeamOrientation::Opposed);
    // A piece sewn to itself answers by the same rule: the top and the bottom
    // of one square run against each other, and left meets left.
    let (top, bottom) = (&spread[0].tracts[0], &spread[0].tracts[2]);
    assert_eq!(facing(top, bottom), SeamOrientation::Opposed);
}

#[test]
fn escape_lets_go_of_the_first_side_and_then_of_the_tool() {
    let (spread, view) = (squares(), View::default());
    let (gesture, _, _) = update(Gesture::Idle, down(RIGHT_OF_FIRST), &spread, view);
    let (gesture, commands, said) = update(gesture, key(Key::Escape), &spread, view);
    assert_eq!(gesture, Gesture::Idle);
    assert!(commands.is_empty());
    assert_eq!(said, Arranged::default(), "no entry was open to cancel");

    let (gesture, commands, said) = update(gesture, key(Key::Escape), &spread, view);
    assert_eq!(gesture, Gesture::Idle);
    assert!(commands.is_empty());
    assert_eq!(
        said.tool,
        Some(Tool::Select),
        "nothing in hand but the tool"
    );
}

#[test]
fn the_same_tract_twice_puts_it_down_and_sews_nothing() {
    let (spread, view) = (squares(), View::default());
    let (gesture, _, _) = update(Gesture::Idle, down(RIGHT_OF_FIRST), &spread, view);
    let (gesture, commands, said) = update(gesture, down(RIGHT_OF_FIRST), &spread, view);
    assert_eq!(gesture, Gesture::Idle);
    assert!(commands.is_empty(), "a tract is not sewn to itself");
    assert_eq!(said.stack, None);
}

#[test]
fn a_press_on_the_bare_mat_slides_the_product_and_keeps_the_side_picked() {
    let (spread, view) = (squares(), View::default());
    let (gesture, _, _) = update(Gesture::Idle, down(RIGHT_OF_FIRST), &spread, view);
    let bare = view.to_screen([20.0, 40.0]);
    let (gesture, _, _) = update(gesture, Input::Down(bare, Mods::default()), &spread, view);
    let run = vec2(30.0, -12.0);
    let moved = Input::Move(bare + run, Mods::default());
    let (gesture, commands, said) = update(gesture, moved, &spread, view);
    assert_eq!(said.pan, run);
    assert!(commands.is_empty());
    let up = Input::Up(bare + run, Mods::default());
    let (gesture, _, _) = update(gesture, up, &spread, view);
    let first = pick(&spread, 0, 1);
    assert_eq!(gesture, Gesture::Sewing(Sewing { first, pan: None }));

    // With no side picked the same press is a plain pan.
    let (gesture, _, _) = update(
        Gesture::Idle,
        Input::Down(bare, Mods::default()),
        &spread,
        view,
    );
    assert_eq!(gesture, Gesture::Pan { from: bare });
}

#[test]
fn a_side_whose_piece_has_gone_is_not_sewn_to_anything() {
    let (spread, view) = (squares(), View::default());
    let (gesture, _, _) = update(Gesture::Idle, down(RIGHT_OF_FIRST), &spread, view);
    let (gesture, commands, _) = update(gesture, down(LEFT_OF_SECOND), &spread[1..], view);
    assert_eq!(gesture, Gesture::Idle);
    assert!(
        commands.is_empty(),
        "the first side names no tract any more"
    );
}

#[test]
fn the_nearest_tract_is_the_one_picked_and_only_within_reach() {
    let spread = squares();
    assert_eq!(under([10.3, 5.0], &spread, 0.5), Some(pick(&spread, 0, 1)));
    assert_eq!(under([29.8, 5.0], &spread, 0.5), Some(pick(&spread, 1, 3)));
    assert_eq!(under([20.0, 5.0], &spread, 0.5), None, "between the two");
    assert_eq!(under([5.0, 5.0], &spread, 0.5), None, "inside a piece");
}

#[test]
fn undo_and_redo_answer_only_with_nothing_in_hand() {
    let (spread, view) = (squares(), View::default());
    let command = Mods {
        command: true,
        ..Mods::default()
    };
    let undo = Input::Key(Key::Z, command);
    let (_, _, said) = update(Gesture::Idle, undo.clone(), &spread, view);
    assert_eq!(said.stack, Some(Stack::Undo));
    let redo = Input::Key(
        Key::Z,
        Mods {
            shift: true,
            ..command
        },
    );
    let (_, _, said) = update(Gesture::Idle, redo, &spread, view);
    assert_eq!(said.stack, Some(Stack::Redo));
    let (gesture, _, _) = update(Gesture::Idle, down(RIGHT_OF_FIRST), &spread, view);
    let (_, _, said) = update(gesture, undo, &spread, view);
    assert_eq!(said.stack, None, "a side in hand is let go of first");
}
