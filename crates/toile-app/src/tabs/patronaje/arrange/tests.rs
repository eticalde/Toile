use eframe::egui::{Key, Pos2, vec2};
use toile_engine::draft::{Command, PieceKey};

use super::*;

/// Two pieces on the whole product: a square where its own coordinates put
/// it, and one the overview drew twenty centimetres across and five up.
fn laid() -> Vec<Laid> {
    let square = |index: u32, [x, y]: [f64; 2]| Laid {
        piece: PieceKey::new(index, 0),
        shift: [x, y],
        outline: vec![[x, y], [x + 10.0, y], [x + 10.0, y + 10.0], [x, y + 10.0]],
    };
    vec![square(0, [0.0, 0.0]), square(1, [20.0, -5.0])]
}

fn on_glass(cm: [f64; 2]) -> Pos2 {
    View::default().to_screen(cm)
}

#[test]
fn a_click_on_a_piece_puts_it_in_front_and_edits_nothing() {
    let (laid, view) = (laid(), View::default());
    let at = on_glass([25.0, 0.0]);
    let (gesture, commands, said) =
        update(Gesture::Idle, Input::Down(at, Mods::default()), &laid, view);
    assert!(commands.is_empty());
    assert_eq!(said.chosen, Some(laid[1].piece));
    assert_eq!(said.stack, Some(Stack::Open(MOVE)));
    let tremor = Input::Move(at + vec2(1.0, 0.0), Mods::default());
    let (gesture, commands, _) = update(gesture, tremor, &laid, view);
    assert!(
        commands.is_empty(),
        "a tremor under a hair is still a click"
    );
    let (gesture, commands, said) = update(gesture, Input::Up(at, Mods::default()), &laid, view);
    assert_eq!(gesture, Gesture::Idle);
    assert!(commands.is_empty());
    assert_eq!(
        said.stack,
        Some(Stack::Close),
        "an empty entry leaves nothing"
    );
}

#[test]
fn a_drag_places_the_piece_from_where_the_overview_drew_it() {
    let (laid, view) = (laid(), View::default());
    let grab = on_glass([25.0, 0.0]);
    let (mut gesture, _, _) = update(
        Gesture::Idle,
        Input::Down(grab, Mods::default()),
        &laid,
        view,
    );
    let mut last = None;
    for step in 1..=3 {
        let step = f64::from(step);
        let at = on_glass([25.0 + 4.0 * step, 2.0 * step]);
        let (next, commands, said) = update(gesture, Input::Move(at, Mods::default()), &laid, view);
        gesture = next;
        assert_eq!(said.stack, None, "one drag, one entry");
        let [
            Command::PlacePiece {
                piece,
                to: Some(to),
            },
        ] = commands.as_slice()
        else {
            panic!("a drag frame places the piece: {commands:?}");
        };
        assert_eq!(*piece, laid[1].piece);
        last = Some(*to);
    }
    let to = last.expect("the drag moved");
    assert!(
        (to.x - 32.0).abs() < 1.0e-9 && (to.y - 1.0).abs() < 1.0e-9,
        "twelve across and six down from where it was drawn: {to:?}"
    );
    let up = Input::Up(on_glass([37.0, 6.0]), Mods::default());
    let (_, commands, said) = update(gesture, up, &laid, view);
    assert!(commands.is_empty());
    assert_eq!(said.stack, Some(Stack::Close));
}

#[test]
fn escape_takes_a_drag_back_but_never_the_entry_before_a_click() {
    let (laid, view) = (laid(), View::default());
    let grab = on_glass([5.0, 5.0]);
    let escape = Input::Key(Key::Escape, Mods::default());
    let (held, _, _) = update(
        Gesture::Idle,
        Input::Down(grab, Mods::default()),
        &laid,
        view,
    );
    let (gesture, _, said) = update(held.clone(), escape.clone(), &laid, view);
    assert_eq!(gesture, Gesture::Idle);
    assert_eq!(
        said.stack,
        Some(Stack::Close),
        "nothing moved, nothing taken back"
    );

    let away = Input::Move(grab + vec2(30.0, 0.0), Mods::default());
    let (moved, commands, _) = update(held, away, &laid, view);
    assert_eq!(commands.len(), 1);
    let (_, _, said) = update(moved, escape, &laid, view);
    assert_eq!(said.stack, Some(Stack::Cancel));
}

#[test]
fn a_press_on_bare_mat_slides_the_product_and_chooses_nothing() {
    let (laid, view) = (laid(), View::default());
    let from = on_glass([60.0, 60.0]);
    let (gesture, commands, said) = update(
        Gesture::Idle,
        Input::Down(from, Mods::default()),
        &laid,
        view,
    );
    assert_eq!(gesture, Gesture::Pan { from });
    assert!(commands.is_empty());
    assert_eq!(said, Arranged::default());
    let slide = Input::Move(from + vec2(12.0, -4.0), Mods::default());
    let (_, _, said) = update(gesture, slide, &laid, view);
    assert_eq!(said.pan, vec2(12.0, -4.0));
}

#[test]
fn the_keyboard_reaches_the_undo_stack() {
    let (laid, view) = (laid(), View::default());
    let command = Mods {
        command: true,
        ..Mods::default()
    };
    let (_, _, said) = update(Gesture::Idle, Input::Key(Key::Z, command), &laid, view);
    assert_eq!(said.stack, Some(Stack::Undo));
    let both = Mods {
        shift: true,
        ..command
    };
    let (_, _, said) = update(Gesture::Idle, Input::Key(Key::Z, both), &laid, view);
    assert_eq!(said.stack, Some(Stack::Redo));
}

#[test]
fn a_placement_that_rounds_to_zero_is_a_plain_zero() {
    assert!(tenths(-0.01).is_sign_positive());
}
