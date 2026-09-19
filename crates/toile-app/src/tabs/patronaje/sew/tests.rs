use eframe::egui::vec2;
use toile_engine::draft::{PieceKey, PointKey, SeamOrientation};

use super::super::tract::Tract;
use super::*;

/// Two ten centimetre squares side by side, twenty apart, both drawn the same
/// way round: along the top, down the right, back along the bottom, up the
/// left.
pub(in crate::tabs::patronaje) fn squares() -> Vec<Spread> {
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

/// A run of one square's tracts by their places in the contour — 0 the top, 1
/// the right side, 2 the bottom, 3 the left side — from the first to the last.
pub(in crate::tabs::patronaje) fn run(spread: &[Spread], piece: usize, tracts: [usize; 2]) -> Pick {
    let of = &spread[piece];
    Pick {
        piece: of.piece,
        from: of.tracts[tracts[0]].node,
        to: of.tracts[tracts[1]].to,
    }
}

/// One tract of one square, as a press picks it.
pub(in crate::tabs::patronaje) fn pick(spread: &[Spread], piece: usize, tract: usize) -> Pick {
    run(spread, piece, [tract, tract])
}

fn seen(spread: &[Spread]) -> Seen<'_> {
    Seen {
        spread,
        view: View::default(),
        chosen: None,
    }
}

fn down(cm: [f64; 2]) -> Input {
    Input::Down(View::default().to_screen(cm), Mods::default())
}

fn shift_down(cm: [f64; 2]) -> Input {
    let shift = Mods {
        shift: true,
        ..Mods::default()
    };
    Input::Down(View::default().to_screen(cm), shift)
}

fn key(key: Key) -> Input {
    Input::Key(key, Mods::default())
}

/// The middles of the first square's top, right side, bottom and left side,
/// and of the second square's left side and bottom.
const TOP: [f64; 2] = [5.0, 0.0];
const RIGHT: [f64; 2] = [10.0, 5.0];
const BOTTOM: [f64; 2] = [5.0, 10.0];
const LEFT: [f64; 2] = [0.0, 5.0];
const SECOND_LEFT: [f64; 2] = [30.0, 5.0];
const SECOND_BOTTOM: [f64; 2] = [35.0, 10.0];

#[test]
fn two_presses_on_two_tracts_sew_them_in_one_entry() {
    let spread = squares();
    let (gesture, commands, said) = update(Gesture::Idle, down(RIGHT), &seen(&spread));
    let first = pick(&spread, 0, 1);
    assert_eq!(gesture, Gesture::Sewing(Sewing { first, pan: None }));
    assert!(commands.is_empty(), "one side is not a seam yet");
    assert_eq!(said.stack, None, "and nothing has been opened for it");
    assert_eq!(said.chosen, Some(spread[0].piece));

    let (gesture, commands, said) = update(gesture, down(SECOND_LEFT), &seen(&spread));
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
fn shift_lengthens_the_side_in_hand_at_either_end_and_the_run_is_sewn_as_one_stretch() {
    let spread = squares();
    let (gesture, _, _) = update(Gesture::Idle, down(RIGHT), &seen(&spread));
    let (gesture, commands, said) = update(gesture, shift_down(BOTTOM), &seen(&spread));
    let first = run(&spread, 0, [1, 2]);
    assert_eq!(gesture, Gesture::Sewing(Sewing { first, pan: None }));
    assert!(commands.is_empty(), "a longer side is still only a side");
    assert_eq!(said.refused, None);
    let (gesture, _, _) = update(gesture, shift_down(TOP), &seen(&spread));
    let first = run(&spread, 0, [0, 2]);
    assert_eq!(
        gesture,
        Gesture::Sewing(Sewing { first, pan: None }),
        "the tract before the head comes in at the head"
    );

    let (_, commands, said) = update(gesture, down(SECOND_LEFT), &seen(&spread));
    assert_eq!(said.stack, Some(Stack::Once(SEW)));
    let [Command::AddSeam { seam, .. }] = commands.as_slice() else {
        panic!("one seam, in one command: {commands:?}");
    };
    assert_eq!(seam.a, first.range(), "three tracts, one stretch");
    assert_eq!(seam.b, pick(&spread, 1, 3).range());
}

#[test]
fn a_tract_that_does_not_touch_the_side_in_hand_is_refused_and_the_side_kept() {
    let spread = squares();
    let (held, _, _) = update(Gesture::Idle, down(RIGHT), &seen(&spread));
    for apart in [LEFT, SECOND_LEFT] {
        let (gesture, commands, said) = update(held.clone(), shift_down(apart), &seen(&spread));
        assert_eq!(gesture, held, "the side picked stays in hand");
        assert!(commands.is_empty());
        assert_eq!(said.refused, Some(APART), "and the bar says why");
    }
}

#[test]
fn shift_on_a_tract_of_the_side_takes_it_back_out() {
    let spread = squares();
    let (gesture, _, _) = update(Gesture::Idle, down(RIGHT), &seen(&spread));
    let (gesture, _, _) = update(gesture, shift_down(BOTTOM), &seen(&spread));
    let (gesture, _, _) = update(gesture, shift_down(RIGHT), &seen(&spread));
    let first = pick(&spread, 0, 2);
    assert_eq!(gesture, Gesture::Sewing(Sewing { first, pan: None }));
    let (gesture, _, said) = update(gesture, shift_down(BOTTOM), &seen(&spread));
    assert_eq!(
        gesture,
        Gesture::Idle,
        "its only tract gone, so is the side"
    );
    assert_eq!(said.refused, None);
}

#[test]
fn shift_with_a_seam_chosen_adjusts_the_side_the_tract_continues() {
    let spread = squares();
    let key = SeamKey::new(0, 0);
    let (a, b) = (pick(&spread, 0, 1), pick(&spread, 1, 3));
    let seam = Seam::plain(a.range(), b.range(), SeamOrientation::Opposed);
    let chosen = Seen {
        chosen: Some((key, seam)),
        ..seen(&spread)
    };
    // The second square's bottom runs into its left side: it comes in at the
    // head of side B, and the seam keeps its key and the way it runs.
    let (gesture, commands, said) = update(Gesture::Idle, shift_down(SECOND_BOTTOM), &chosen);
    assert_eq!(gesture, Gesture::Idle);
    assert_eq!(said.stack, Some(Stack::Once(RESIDE)));
    let longer = Seam {
        b: run(&spread, 1, [2, 3]).range(),
        ..seam
    };
    assert_eq!(commands, resewn(key, longer));

    let (_, commands, _) = update(Gesture::Idle, shift_down(BOTTOM), &chosen);
    let longer = Seam {
        a: run(&spread, 0, [1, 2]).range(),
        ..seam
    };
    assert_eq!(commands, resewn(key, longer), "side A answers for its own");

    let (_, commands, said) = update(Gesture::Idle, shift_down(LEFT), &chosen);
    assert!(commands.is_empty());
    assert_eq!(said.refused, Some(APART), "a tract that continues neither");
    let (_, commands, said) = update(Gesture::Idle, shift_down(RIGHT), &chosen);
    assert!(commands.is_empty());
    assert_eq!(said.refused, Some(LAST), "a sewn side keeps a tract");
}

#[test]
fn escape_lets_go_of_the_first_side_and_then_of_the_tool() {
    let spread = squares();
    let (gesture, _, _) = update(Gesture::Idle, down(RIGHT), &seen(&spread));
    let (gesture, commands, said) = update(gesture, key(Key::Escape), &seen(&spread));
    assert_eq!(gesture, Gesture::Idle);
    assert!(commands.is_empty());
    assert_eq!(said, Arranged::default(), "no entry was open to cancel");

    let (gesture, commands, said) = update(gesture, key(Key::Escape), &seen(&spread));
    assert_eq!(gesture, Gesture::Idle);
    assert!(commands.is_empty());
    assert_eq!(said.tool, Some(Tool::Select), "only the tool was in hand");
}

#[test]
fn a_tract_of_the_side_in_hand_puts_it_down_and_sews_nothing() {
    let spread = squares();
    let (gesture, _, _) = update(Gesture::Idle, down(RIGHT), &seen(&spread));
    let (gesture, commands, said) = update(gesture, down(RIGHT), &seen(&spread));
    assert_eq!(gesture, Gesture::Idle);
    assert!(commands.is_empty(), "a tract is not sewn to itself");
    assert_eq!(said.stack, None);

    let (gesture, _, _) = update(Gesture::Idle, down(RIGHT), &seen(&spread));
    let (gesture, _, _) = update(gesture, shift_down(BOTTOM), &seen(&spread));
    let (gesture, commands, _) = update(gesture, down(BOTTOM), &seen(&spread));
    assert_eq!(gesture, Gesture::Idle);
    assert!(commands.is_empty(), "nor a run to a part of itself");
}

#[test]
fn a_press_on_the_bare_mat_slides_the_product_and_keeps_the_side_picked() {
    let spread = squares();
    let (gesture, _, _) = update(Gesture::Idle, down(RIGHT), &seen(&spread));
    let bare = View::default().to_screen([20.0, 40.0]);
    let press = Input::Down(bare, Mods::default());
    let (gesture, _, _) = update(gesture, press.clone(), &seen(&spread));
    let slid = vec2(30.0, -12.0);
    let moved = Input::Move(bare + slid, Mods::default());
    let (gesture, commands, said) = update(gesture, moved, &seen(&spread));
    assert_eq!(said.pan, slid);
    assert!(commands.is_empty());
    let up = Input::Up(bare + slid, Mods::default());
    let (gesture, _, _) = update(gesture, up, &seen(&spread));
    let first = pick(&spread, 0, 1);
    assert_eq!(gesture, Gesture::Sewing(Sewing { first, pan: None }));

    // With no side picked the same press is a plain pan.
    let (gesture, _, _) = update(Gesture::Idle, press, &seen(&spread));
    assert_eq!(gesture, Gesture::Pan { from: bare });
}

#[test]
fn a_side_whose_piece_has_gone_is_not_sewn_to_anything() {
    let spread = squares();
    let (gesture, _, _) = update(Gesture::Idle, down(RIGHT), &seen(&spread));
    let (gesture, commands, _) = update(gesture, down(SECOND_LEFT), &seen(&spread[1..]));
    assert_eq!(gesture, Gesture::Idle);
    assert!(commands.is_empty(), "the first side names no tract now");
}

#[test]
fn undo_and_redo_answer_only_with_nothing_in_hand() {
    let spread = squares();
    let command = Mods {
        command: true,
        ..Mods::default()
    };
    let undo = Input::Key(Key::Z, command);
    let (_, _, said) = update(Gesture::Idle, undo.clone(), &seen(&spread));
    assert_eq!(said.stack, Some(Stack::Undo));
    let shifted = Mods {
        shift: true,
        ..command
    };
    let (_, _, said) = update(Gesture::Idle, Input::Key(Key::Z, shifted), &seen(&spread));
    assert_eq!(said.stack, Some(Stack::Redo));
    let (gesture, _, _) = update(Gesture::Idle, down(RIGHT), &seen(&spread));
    let (_, _, said) = update(gesture, undo, &seen(&spread));
    assert_eq!(said.stack, None, "a side in hand is let go of first");
}
