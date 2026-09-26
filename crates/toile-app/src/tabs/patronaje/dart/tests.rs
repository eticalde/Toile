use eframe::egui::Key;
use toile_engine::draft::{Binding, ChangeClass, Doc, FoldDirection, Segment};

use super::*;
use crate::tabs::patronaje::input::tests::{Table, table};
use crate::tabs::patronaje::state::Tool;
use crate::tabs::patronaje::view::View;

/// The waistline of the block's front: the tract leaving its first node, which
/// runs twenty-two centimetres straight along the top of the piece.
const WAIST: usize = 0;

fn down(at: Pos2) -> Input {
    Input::Down(at, Mods::default())
}

fn key(pressed: Key) -> Input {
    Input::Key(pressed, Mods::default())
}

/// The wedge a gesture holds, out of whatever the reducer answered with.
fn darting(gesture: &Gesture) -> Darting {
    match gesture {
        Gesture::Darting(held) => held.clone(),
        other => panic!("a wedge is being cut: {other:?}"),
    }
}

/// A fraction of the way along one straight tract of the piece, on the glass.
fn along(table: &Table, tract: usize, t: f64) -> Pos2 {
    let from = table.nodes[tract].1;
    let to = table.nodes[(tract + 1) % table.nodes.len()].1;
    View::default().to_screen([0, 1].map(|axis| from[axis] + (to[axis] - from[axis]) * t))
}

/// A place on the drawn line of one tract, which for a bent one is not its
/// chord.
fn on_tract(table: &Table, tract: usize) -> Pos2 {
    let line = &table.tracts[tract].line;
    View::default().to_screen(line[line.len() / 2])
}

/// A place well inside the paper of the block, clear of every tract.
fn in_the_cloth(table: &Table) -> Pos2 {
    let corner = table.nodes[WAIST].1;
    View::default().to_screen([corner[0] + 5.0, corner[1] + 5.0])
}

/// The tract of the block that bends, out of the ones its curves are on.
fn bent(table: &Table) -> usize {
    let node = table.bent().expect("the block draws a curve").node;
    table
        .nodes
        .iter()
        .position(|&(key, _)| key == node)
        .expect("a handle hangs off a node of the contour")
}

/// The whole gesture: two legs on the waistline and the apex in the cloth.
fn cut_one(table: &Table, legs: (f64, f64)) -> (Vec<Command>, Feedback) {
    let ctx = table.wielding(Tool::Dart);
    let (gesture, commands, _) = begin(along(table, WAIST, legs.0), Mods::default(), &ctx);
    assert!(commands.is_empty(), "one leg is not a wedge");
    assert_eq!(darting(&gesture).legs.len(), 1);
    let second = down(along(table, WAIST, legs.1));
    let (gesture, commands, _) = update(darting(&gesture), &second, &ctx);
    assert!(commands.is_empty(), "two legs are not a wedge either");
    assert_eq!(darting(&gesture).legs.len(), LEGS);
    let apex = down(in_the_cloth(table));
    let (gesture, commands, feedback) = update(darting(&gesture), &apex, &ctx);
    assert_eq!(gesture, Gesture::Idle, "the wedge is cut and let go of");
    (commands, feedback)
}

/// The dart and the wedge one command carries.
fn asked(commands: &[Command]) -> (Dart, DartWedge) {
    assert_eq!(commands.len(), 1, "one dart, one command: {commands:?}");
    let Command::AddDart {
        identity,
        dart,
        wedge,
    } = &commands[0]
    else {
        panic!("the tool cuts a dart: {:?}", commands[0]);
    };
    assert_eq!(*identity, Identity::New);
    (*dart, (**wedge).clone())
}

/// Where the three nodes of a wedge are written, in centimetres.
fn places(wedge: &DartWedge) -> [[f64; 2]; 3] {
    let literal = |binding: &Binding| match binding {
        Binding::Literal(cm) => *cm,
        Binding::Formula(_) => panic!("the tool writes the place the press landed on"),
    };
    [0, 1, 2].map(|at| {
        let value = &wedge.nodes[at].value;
        [literal(&value.x), literal(&value.y)]
    })
}

/// Three presses make a dart: two legs on one tract of the contour and the apex
/// in the cloth, as one entry of the history.
#[test]
fn two_legs_and_an_apex_cut_a_wedge_after_the_node_the_tract_leaves() {
    let table = table();
    let (commands, feedback) = cut_one(&table, (0.3, 0.7));
    let (dart, wedge) = asked(&commands);
    assert_eq!(wedge.piece, table.piece);
    assert_eq!(wedge.after, Some(table.nodes[WAIST].0));
    for node in &wedge.nodes {
        assert_eq!(node.segment, Segment::Line, "a wedge is cut straight");
        assert_eq!(node.identity, Identity::New);
    }
    // In contour order: the leg nearer the node the tract leaves, the apex, the
    // other leg. The waistline runs along the top of the piece, so the apex is
    // the one of the three that is off it.
    let at = places(&wedge);
    assert!(at[0][0] < at[2][0], "the legs are in contour order: {at:?}");
    assert!(at[1][1] > at[0][1], "the apex is the deep one: {at:?}");
    assert_eq!(feedback.stack, Some(Stack::Once(PUT_ON)));
    assert_eq!(
        feedback.select,
        Some(Selection::point(dart.apex)),
        "the panel opens on the dart the hand just cut"
    );
}

/// A dart the tool cuts is one the document takes, and the keys the command
/// predicted are the keys the edit issues.
///
/// That is what lets the panel open on a dart that did not exist when the
/// gesture wrote the command down.
#[test]
fn the_dart_the_tool_cuts_is_one_the_document_takes() {
    let table = table();
    let (commands, _) = cut_one(&table, (0.3, 0.7));
    let (dart, _) = asked(&commands);
    let mut doc: Doc = table.draft.doc().clone();
    let nodes = |doc: &Doc| {
        doc.pieces
            .get(table.piece)
            .expect("it is there")
            .contour
            .len()
    };
    let before = nodes(&doc);
    let applied = commands
        .into_iter()
        .next()
        .expect("one command")
        .apply(&mut doc)
        .expect("the wedge the tool aimed is one the contour takes");
    assert_eq!(applied.class, ChangeClass::Topology);
    assert_eq!(applied.touched, vec![table.piece]);
    assert_eq!(nodes(&doc), before + 3, "the wedge brings three nodes");
    let (_, held) = doc.darts.iter().next().expect("the cut wrote one");
    assert_eq!(held.apex, dart.apex);
    assert_eq!(held.legs, dart.legs);
    assert_eq!(held.seam, dart.seam);
    assert_eq!(held.fold, FoldDirection::TowardStart);
    assert!(doc.seams.get(held.seam).is_some(), "the seam closes it");
}

/// Which way the wedge is pressed follows from the leg pressed first, so the
/// two orders of the same two places are two different darts.
#[test]
fn the_wedge_is_pressed_toward_the_leg_pressed_first() {
    let table = table();
    let (near_first, _) = cut_one(&table, (0.3, 0.7));
    assert_eq!(asked(&near_first).0.fold, FoldDirection::TowardStart);
    let (far_first, _) = cut_one(&table, (0.7, 0.3));
    assert_eq!(asked(&far_first).0.fold, FoldDirection::TowardEnd);
    // And the wedge itself is the same either way: the legs go into the contour
    // in contour order whichever of them the hand reached for first.
    assert_eq!(places(&asked(&near_first).1), places(&asked(&far_first).1));
}

/// A press that lands off the contour puts no leg, and says so where the person
/// reads it rather than leaving the mat silent.
#[test]
fn a_press_off_the_contour_puts_no_leg_and_says_why() {
    let table = table();
    let ctx = table.wielding(Tool::Dart);
    let (gesture, commands, feedback) = begin(in_the_cloth(&table), Mods::default(), &ctx);
    assert_eq!(gesture, Gesture::Idle);
    assert!(commands.is_empty());
    assert_eq!(feedback.refused, Some(OFF_EDGE));
}

/// Both legs go on one tract: the three nodes of a wedge stand together in the
/// contour, so legs on two tracts would swallow every node between them.
#[test]
fn the_second_leg_has_to_be_on_the_tract_the_first_is_on() {
    let table = table();
    let ctx = table.wielding(Tool::Dart);
    let (gesture, _, _) = begin(along(&table, WAIST, 0.3), Mods::default(), &ctx);
    let elsewhere = down(on_tract(&table, WAIST + 1));
    let (gesture, commands, feedback) = update(darting(&gesture), &elsewhere, &ctx);
    assert!(commands.is_empty());
    assert_eq!(feedback.refused, Some(OTHER_TRACT));
    assert_eq!(darting(&gesture).legs.len(), 1, "the first leg is kept");
}

/// A tract that bends takes no wedge, and is refused rather than straightened.
#[test]
fn a_bending_tract_takes_no_wedge() {
    let table = table();
    let ctx = table.wielding(Tool::Dart);
    let curve = bent(&table);
    let (gesture, commands, feedback) = begin(on_tract(&table, curve), Mods::default(), &ctx);
    assert_eq!(gesture, Gesture::Idle);
    assert!(commands.is_empty());
    assert_eq!(feedback.refused, Some(BENT));
}

/// The apex goes inside the cloth: one caught on the contour would be a wedge
/// with no width at its deep end.
#[test]
fn an_apex_on_the_contour_is_refused() {
    let table = table();
    let ctx = table.wielding(Tool::Dart);
    let (gesture, _, _) = begin(along(&table, WAIST, 0.3), Mods::default(), &ctx);
    let second = down(along(&table, WAIST, 0.7));
    let (gesture, _, _) = update(darting(&gesture), &second, &ctx);
    let on_the_edge = down(along(&table, WAIST, 0.5));
    let (gesture, commands, feedback) = update(darting(&gesture), &on_the_edge, &ctx);
    assert!(commands.is_empty());
    assert_eq!(feedback.refused, Some(OFF_CLOTH));
    assert_eq!(darting(&gesture).legs.len(), LEGS, "both legs are kept");
}

/// Nothing of a wedge reaches the document until the apex goes down, so both
/// ways out of it are free.
#[test]
fn a_wedge_is_walked_away_from_with_nothing_to_unwind() {
    let table = table();
    let ctx = table.wielding(Tool::Dart);
    let (gesture, _, _) = begin(along(&table, WAIST, 0.3), Mods::default(), &ctx);
    let second = down(along(&table, WAIST, 0.7));
    let (gesture, _, _) = update(darting(&gesture), &second, &ctx);
    let (gesture, commands, _) = update(darting(&gesture), &key(Key::Backspace), &ctx);
    assert!(commands.is_empty());
    assert_eq!(darting(&gesture).legs.len(), 1, "one leg taken back");
    let (gesture, commands, feedback) = update(darting(&gesture), &key(Key::Backspace), &ctx);
    assert_eq!(gesture, Gesture::Idle, "and the last one with it");
    assert!(commands.is_empty());
    assert_eq!(feedback.stack, None);
    let (gesture, _, _) = begin(along(&table, WAIST, 0.3), Mods::default(), &ctx);
    let (gesture, commands, feedback) = update(darting(&gesture), &key(Key::Escape), &ctx);
    assert_eq!(gesture, Gesture::Idle);
    assert!(commands.is_empty());
    assert_eq!(feedback.stack, None);
}
