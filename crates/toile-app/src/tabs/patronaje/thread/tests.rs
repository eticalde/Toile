#![allow(
    clippy::float_cmp,
    reason = "every place here is a whole centimetre, exact in a double"
)]

use super::super::gesture::Mods;
use super::super::sew::Pick;
use super::super::sew::tests::{pick, run, squares};
use super::*;

/// The right side of the first square sewn to the left side of the second.
fn across(spread: &[Spread], orientation: SeamOrientation) -> Seam {
    let (right, left) = (pick(spread, 0, 1), pick(spread, 1, 3));
    Seam::plain(right.range(), left.range(), orientation)
}

/// That seam and one along the two bottoms, as the mat draws them.
fn threads(spread: &[Spread]) -> Vec<Thread> {
    let bottoms = [pick(spread, 0, 2), pick(spread, 1, 2)].map(Pick::range);
    let seams = [
        across(spread, SeamOrientation::Opposed),
        Seam::plain(bottoms[0], bottoms[1], SeamOrientation::Aligned),
    ];
    seams
        .iter()
        .enumerate()
        .map(|(index, seam)| Thread {
            seam: SeamKey::new(index as u32, 0),
            ordinal: index + 1,
            sides: sides(spread, seam).expect("both sides are on the mat"),
            dart: false,
        })
        .collect()
}

fn reach(threads: &[Thread], tool: Tool, chosen: Option<SeamKey>) -> Reach<'_> {
    Reach {
        threads,
        view: View::default(),
        tool,
        chosen,
    }
}

fn down(cm: [f64; 2]) -> Input {
    Input::Down(View::default().to_screen(cm), Mods::default())
}

fn key(key: Key) -> Input {
    Input::Key(key, Mods::default())
}

#[test]
fn an_opposed_seam_draws_its_second_side_tail_to_head() {
    let spread = squares();
    // The right side of the first runs down; the left side of the second up.
    let [a, b] = sides(&spread, &across(&spread, SeamOrientation::Aligned)).expect("on the mat");
    assert_eq!(a, vec![[10.0, 0.0], [10.0, 10.0]]);
    assert_eq!(b, vec![[30.0, 10.0], [30.0, 0.0]], "as its contour runs");

    let [a, b] = sides(&spread, &across(&spread, SeamOrientation::Opposed)).expect("on the mat");
    assert_eq!(a, vec![[10.0, 0.0], [10.0, 10.0]], "side A never turns");
    assert_eq!(
        b,
        vec![[30.0, 0.0], [30.0, 10.0]],
        "the pairing walks it from its tail, top first, like side A"
    );
}

#[test]
fn a_side_of_several_tracts_is_one_line_from_its_head_to_its_tail() {
    let spread = squares();
    let seam = Seam {
        a: run(&spread, 0, [1, 3]).range(),
        ..across(&spread, SeamOrientation::Opposed)
    };
    let [a, _] = sides(&spread, &seam).expect("on the mat");
    assert_eq!(a.first(), Some(&[10.0, 0.0]));
    assert_eq!(a.last(), Some(&[0.0, 0.0]));
    assert!(
        a.contains(&[10.0, 10.0]) && a.contains(&[0.0, 10.0]),
        "{a:?}"
    );
}

#[test]
fn a_seam_with_a_side_of_no_length_draws_no_thread() {
    let spread = squares();
    let of = &spread[1];
    let pinched = EdgeRange::between(of.piece, of.tracts[3].node, of.tracts[3].node);
    let seam = Seam {
        b: pinched,
        ..across(&spread, SeamOrientation::Aligned)
    };
    assert_eq!(
        sides(&spread, &seam),
        None,
        "the engine pairs nothing there"
    );
}

#[test]
fn a_press_on_a_thread_chooses_its_seam_and_a_second_one_lets_go() {
    let spread = squares();
    let threads = threads(&spread);
    let (first, second) = (threads[0].seam, threads[1].seam);
    let none = reach(&threads, Tool::Select, None);
    let (gesture, commands, said) =
        update(&Gesture::Idle, &down([30.2, 4.0]), &none).expect("on side B of the first");
    assert_eq!(gesture, Gesture::Idle, "the piece under it is not taken up");
    assert!(commands.is_empty(), "choosing writes nothing");
    assert_eq!(said.select, Some(Selection::Seam(first)));

    let held = reach(&threads, Tool::Select, Some(first));
    let (_, _, said) = update(&Gesture::Idle, &down([5.0, 10.1]), &held).expect("a bottom");
    assert_eq!(said.select, Some(Selection::Seam(second)));
    let (_, _, said) = update(&Gesture::Idle, &down([10.0, 5.0]), &held).expect("side A");
    assert_eq!(said.select, Some(Selection::None), "the one already chosen");
}

#[test]
fn a_press_off_every_thread_or_with_the_sewing_tool_is_not_the_seams_to_answer() {
    let spread = squares();
    let threads = threads(&spread);
    let select = reach(&threads, Tool::Select, None);
    assert_eq!(update(&Gesture::Idle, &down([5.0, 5.0]), &select), None);
    assert_eq!(update(&Gesture::Idle, &down([5.0, 0.0]), &select), None);
    let sewing = reach(&threads, Tool::Sew, None);
    assert_eq!(
        update(&Gesture::Idle, &down([10.0, 5.0]), &sewing),
        None,
        "that press picks a tract"
    );
    let space = Mods {
        space: true,
        ..Mods::default()
    };
    let pan = Input::Down(View::default().to_screen([10.0, 5.0]), space);
    assert_eq!(update(&Gesture::Idle, &pan, &select), None);
}

#[test]
fn delete_unpicks_the_chosen_seam_in_one_entry_under_either_tool() {
    let spread = squares();
    let threads = threads(&spread);
    let seam = threads[1].seam;
    for tool in [Tool::Select, Tool::Sew] {
        for pressed in [Key::Delete, Key::Backspace] {
            let held = reach(&threads, tool, Some(seam));
            let (gesture, commands, said) =
                update(&Gesture::Idle, &key(pressed), &held).expect("a seam is chosen");
            assert_eq!(gesture, Gesture::Idle);
            assert_eq!(commands, vec![Command::RemoveSeam { seam }]);
            assert_eq!(said.stack, Some(Stack::Once(UNPICK)));
        }
    }
    let none = reach(&threads, Tool::Select, None);
    assert_eq!(update(&Gesture::Idle, &key(Key::Delete), &none), None);
    let busy = Gesture::Pan {
        from: View::default().to_screen([0.0, 0.0]),
    };
    let held = reach(&threads, Tool::Select, Some(seam));
    assert_eq!(update(&busy, &key(Key::Delete), &held), None);
}

/// A dart's own thread is not one the mat lets a person pull.
///
/// The document refuses it as well, and in the language a library speaks. Said
/// here, where the press is, the person reads the way out instead of the rule.
#[test]
fn delete_on_the_thread_that_shuts_a_dart_says_so_and_sends_nothing() {
    let spread = squares();
    let mut threads = threads(&spread);
    threads[1].dart = true;
    let held = reach(&threads, Tool::Select, Some(threads[1].seam));
    let (gesture, commands, said) =
        update(&Gesture::Idle, &key(Key::Delete), &held).expect("a seam is chosen");
    assert_eq!(gesture, Gesture::Idle);
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(said.refused, Some(SHUT));
    assert_eq!(said.stack, None, "and no entry is opened");
}

#[test]
fn escape_lets_go_of_the_chosen_seam() {
    let spread = squares();
    let threads = threads(&spread);
    let held = reach(&threads, Tool::Select, Some(threads[0].seam));
    let (_, commands, said) = update(&Gesture::Idle, &key(Key::Escape), &held).expect("chosen");
    assert!(commands.is_empty());
    assert_eq!(said.select, Some(Selection::None));
    let sewing = reach(&threads, Tool::Sew, Some(threads[0].seam));
    assert_eq!(
        update(&Gesture::Idle, &key(Key::Escape), &sewing),
        None,
        "the sewing tool has the tool itself to let go of first"
    );
}
