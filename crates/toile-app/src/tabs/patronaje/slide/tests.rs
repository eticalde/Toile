#![allow(
    clippy::float_cmp,
    reason = "a fraction written at a step of its own is that number exactly"
)]

use eframe::egui::{Key, vec2};
use toile_engine::draft::{Doc, EdgeAnchor, Identity, Notch, NotchKey, PieceKey, PointKey, block};

use super::*;
use crate::tabs::patronaje::gesture::Mods;
use crate::tabs::patronaje::input::tests::{Table, table_of};
use crate::tabs::patronaje::snap::SnapConfig;
use crate::tabs::patronaje::state::{Selection, Tool};
use crate::tabs::patronaje::view::View;

/// Where along its tract the notch the table carries is cut.
const MIDDLE: f64 = 0.5;

/// The block with one notch cut half way along the tract leaving `label`.
fn marked(label: &str) -> (Table, NotchKey, PointKey) {
    marked_at(label, MIDDLE)
}

/// The same, with the mark cut at whatever fraction of that tract.
fn marked_at(label: &str, t: f64) -> (Table, NotchKey, PointKey) {
    let mut doc = block::trouser_front();
    let piece = doc.piece_named(block::FRONT).expect("the block draws one");
    let from = doc
        .shows_label(piece, label)
        .unwrap_or_else(|| panic!("the block names {label}"));
    let notch = cut(&mut doc, piece, from, t);
    (table_of(doc), notch, from)
}

fn cut(doc: &mut Doc, piece: PieceKey, from: PointKey, t: f64) -> NotchKey {
    let applied = Command::AddNotch {
        identity: Identity::New,
        notch: Notch::lone(EdgeAnchor { piece, from, t }),
        mate: None,
    }
    .apply(doc)
    .expect("the middle of a tract is a place on the contour");
    let Command::RemoveNotch { notch } = applied.inverse else {
        panic!("the inverse takes the mark off the contour");
    };
    notch
}

/// The notch a slide is writing, out of the one command a frame emits.
fn slid(commands: &[Command]) -> EdgeAnchor {
    assert_eq!(commands.len(), 1, "one frame, one edit: {commands:?}");
    let Command::MoveNotch { to, .. } = commands[0] else {
        panic!("a slide moves a notch: {:?}", commands[0]);
    };
    to
}

/// A press on the mark takes it in hand and opens one entry, writing nothing.
#[test]
fn a_press_on_a_notch_takes_it_in_hand_and_writes_nothing() {
    let (table, notch, from) = marked("cintura_cf");
    let ctx = table.holding(SnapConfig::default(), Selection::None);
    let tick = table.ticks()[0];
    let at = View::default().to_screen(tick.at);
    let (gesture, commands, feedback) = grab(notch, at, &ctx);
    assert_eq!(
        gesture,
        Gesture::Sliding(Slide {
            notch,
            grab: at,
            moved: false
        })
    );
    assert!(commands.is_empty(), "a press is not a slide");
    assert_eq!(feedback.stack, Some(Stack::Open(SLIDE)));
    assert_eq!(
        feedback.select,
        Some(Selection::Edge(from)),
        "the tract it was cut into is chosen, so the panel says how far along"
    );
}

/// The mark a press lands on is the one nearest it, and a press away from every
/// mark lands on none.
#[test]
fn the_notch_under_the_pointer_is_the_nearest_one() {
    let (table, notch, _) = marked("cintura_cf");
    let tick = table.ticks()[0];
    assert_eq!(under(tick.at, table.ticks(), 0.5), Some(notch));
    let off = [tick.at[0], tick.at[1] + 5.0];
    assert_eq!(under(off, table.ticks(), 0.5), None, "five centimetres off");
    assert_eq!(under(off, table.ticks(), 6.0), Some(notch), "a wide reach");
}

/// A drag writes one `MoveNotch` per frame, always on the tract the mark was
/// cut into, and the fraction follows the pointer along it.
#[test]
fn a_drag_slides_the_mark_along_its_own_tract() {
    let (table, notch, from) = marked("cintura_cf");
    let ctx = table.holding(SnapConfig::default(), Selection::None);
    let tick = table.ticks()[0];
    let grabbed = View::default().to_screen(tick.at);
    let (gesture, _, _) = grab(notch, grabbed, &ctx);
    let Gesture::Sliding(held) = gesture else {
        panic!("the mark is in hand");
    };
    // The waist runs from the centre front to the side, so a pointer moved
    // toward the side walks the fraction up.
    let along = View::default().to_screen([tick.at[0] + 5.0, tick.at[1] + 2.0]);
    let (gesture, commands, feedback) = update(held, &Input::Move(along, Mods::default()), &ctx);
    let to = slid(&commands);
    assert_eq!(to.piece, table.piece, "the mark stays on its piece");
    assert_eq!(to.from, from, "and on the tract it was cut into");
    assert!(to.t > MIDDLE, "it followed the pointer: {}", to.t);
    assert!(matches!(gesture, Gesture::Sliding(held) if held.moved));
    assert_eq!(feedback.stack, None, "one gesture, one entry");
}

/// However far the pointer wanders, the mark does not leave its tract: the
/// fraction is clamped at the ends, so the mark stops at the node.
#[test]
fn the_mark_stops_at_the_ends_of_its_tract() {
    let (table, notch, _) = marked("cintura_cf");
    let ctx = table.holding(SnapConfig::default(), Selection::None);
    let tick = table.ticks()[0];
    let grabbed = View::default().to_screen(tick.at);
    let held = Slide {
        notch,
        grab: grabbed,
        moved: true,
    };
    for (away, expected) in [(-400.0, 0.0), (400.0, 1.0)] {
        let at = grabbed + vec2(away, 0.0);
        let (_, commands, _) = update(held, &Input::Move(at, Mods::default()), &ctx);
        assert_eq!(slid(&commands).t, expected);
    }
}

/// A tremor under a hair is a click, and a click writes nothing.
#[test]
fn a_tremor_under_a_hair_writes_nothing() {
    let (table, notch, _) = marked("cintura_cf");
    let ctx = table.holding(SnapConfig::default(), Selection::None);
    let grabbed = View::default().to_screen(table.ticks()[0].at);
    let held = Slide {
        notch,
        grab: grabbed,
        moved: false,
    };
    let (gesture, commands, _) = update(
        held,
        &Input::Move(grabbed + vec2(1.0, 0.0), Mods::default()),
        &ctx,
    );
    assert!(commands.is_empty());
    assert!(matches!(gesture, Gesture::Sliding(held) if !held.moved));
}

/// Letting go closes the entry; Escape after a real slide refuses it, and
/// Escape on a press that never left just closes what it opened.
#[test]
fn letting_go_closes_the_entry_and_escape_refuses_the_slide() {
    let (table, notch, _) = marked("cintura_cf");
    let ctx = table.holding(SnapConfig::default(), Selection::None);
    let grabbed = View::default().to_screen(table.ticks()[0].at);
    let held = |moved| Slide {
        notch,
        grab: grabbed,
        moved,
    };
    let up = Input::Up(grabbed, Mods::default());
    let (gesture, commands, feedback) = update(held(true), &up, &ctx);
    assert_eq!(gesture, Gesture::Idle);
    assert!(commands.is_empty(), "the frames did the writing");
    assert_eq!(feedback.stack, Some(Stack::Close));

    let escape = Input::Key(Key::Escape, Mods::default());
    let (_, _, feedback) = update(held(true), &escape, &ctx);
    assert_eq!(feedback.stack, Some(Stack::Cancel));
    let (_, _, feedback) = update(held(false), &escape, &ctx);
    assert_eq!(feedback.stack, Some(Stack::Close));
}

/// A mark whose tract the contour no longer runs through has nowhere to go, so
/// the gesture ends and the entry closes instead of guessing at a place.
#[test]
fn a_mark_whose_tract_has_gone_ends_the_gesture() {
    let mut doc = block::trouser_front();
    let piece = doc.piece_named(block::FRONT).expect("the block draws one");
    let notch = doc.notches.insert(Notch::lone(EdgeAnchor::at_node(
        piece,
        PointKey::new(999, 0),
    )));
    let table = table_of(doc);
    let ctx = table.holding(SnapConfig::default(), Selection::None);
    let held = Slide {
        notch,
        grab: Pos2::ZERO,
        moved: true,
    };
    let at = Pos2::ZERO + vec2(40.0, 40.0);
    let (gesture, commands, feedback) = update(held, &Input::Move(at, Mods::default()), &ctx);
    assert_eq!(gesture, Gesture::Idle);
    assert!(commands.is_empty());
    assert_eq!(feedback.stack, Some(Stack::Close));
}

/// A press on a notch is answered only by the choosing tool, and it is answered
/// before the tract underneath it: the mark is the smaller, more deliberate
/// target of the two.
#[test]
fn a_press_on_a_notch_takes_the_mark_and_not_the_tract_under_it() {
    let (table, notch, _) = marked("cintura_cf");
    let ctx = table.holding(SnapConfig::default(), Selection::None);
    let at = View::default().to_screen(table.ticks()[0].at);
    let (gesture, _, feedback) = crate::tabs::patronaje::input::update(
        Gesture::Idle,
        Input::Down(at, Mods::default()),
        &ctx,
    );
    assert!(matches!(gesture, Gesture::Sliding(held) if held.notch == notch));
    assert_eq!(feedback.stack, Some(Stack::Open(SLIDE)));

    let drawing = EditContext {
        tool: Tool::Point,
        ..table.holding(SnapConfig::default(), Selection::None)
    };
    let (other, _, _) = crate::tabs::patronaje::input::update(
        Gesture::Idle,
        Input::Down(at, Mods::default()),
        &drawing,
    );
    assert!(
        !matches!(other, Gesture::Sliding(_)),
        "a press with Punto in hand is aimed at the contour: {other:?}"
    );
}

/// A mark cut at a node answers a press only while its own tract is chosen.
///
/// Every notch that comes in from Seamly is cut at a node, because there a
/// notch is written on an outline node. The mat draws its cut centred on the
/// node's own dot, so the press that means the mark and the press that means
/// the node land on the same place: with nothing chosen the node has it — it is
/// drawn over the mark, and it is the cut line — and with the mark's own tract
/// chosen the mark does.
#[test]
fn a_mark_cut_at_a_node_answers_the_press_only_while_its_tract_is_chosen() {
    let (table, notch, from) = marked_at("cintura_cf", 0.0);
    let tick = table.ticks()[0];
    let node = table
        .nodes
        .iter()
        .find_map(|&(key, at)| (key == from).then_some(at))
        .expect("the tract leaves a node of the piece");
    assert_eq!(tick.at, node, "the cut is drawn on the node itself");
    let at = View::default().to_screen(tick.at);
    let press = |ctx: &EditContext<'_>| {
        crate::tabs::patronaje::input::update(Gesture::Idle, Input::Down(at, Mods::default()), ctx)
    };

    let loose = table.holding(SnapConfig::default(), Selection::None);
    let (gesture, commands, feedback) = press(&loose);
    assert!(
        matches!(gesture, Gesture::Drag(_)),
        "the node keeps the press: {gesture:?}"
    );
    assert!(commands.is_empty(), "a press is not yet an edit");
    assert_eq!(feedback.select, Some(Selection::point(from)));

    let chosen = table.holding(SnapConfig::default(), Selection::Edge(from));
    let (gesture, commands, feedback) = press(&chosen);
    assert!(matches!(gesture, Gesture::Sliding(held) if held.notch == notch));
    assert!(commands.is_empty(), "and it writes nothing either");
    assert_eq!(feedback.stack, Some(Stack::Open(SLIDE)));

    // And it is the chosen tract's own marks that answer, no others': the
    // choice says which tract is being worked on, not that every mark is live.
    let other = table.nodes[1].0;
    let elsewhere = table.holding(SnapConfig::default(), Selection::Edge(other));
    let (gesture, _, _) = press(&elsewhere);
    assert!(
        matches!(gesture, Gesture::Drag(_)),
        "another tract's choice reaches nothing here: {gesture:?}"
    );
}
