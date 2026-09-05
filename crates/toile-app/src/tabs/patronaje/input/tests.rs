use eframe::egui::{Pos2, vec2};
use toile_engine::draft::{Binding, Draft, PieceKey, PointKey, block};

use super::*;
use crate::tabs::patronaje::snap::{SnapConfig, SnapKind};
use crate::tabs::patronaje::view::View;

/// The block on the table, with its nodes already resolved.
pub(super) struct Table {
    pub(super) draft: Draft,
    piece: PieceKey,
    pub(super) nodes: Vec<(PointKey, [f64; 2])>,
}

pub(super) fn table() -> Table {
    let draft = Draft::from_doc(block::trouser_front()).expect("the block resolves");
    let piece = draft
        .doc()
        .piece_named(block::FRONT)
        .expect("the block draws one piece");
    let nodes = draft.points_cm(piece).to_vec();
    Table {
        draft,
        piece,
        nodes,
    }
}

impl Table {
    /// The context a gesture reduces against, with the snap in whatever state
    /// the test needs it and nothing chosen.
    pub(super) fn context(&self, snap: SnapConfig) -> EditContext<'_> {
        self.holding(snap, Selection::None)
    }

    /// The same, with `chosen` already in hand.
    pub(super) fn holding(&self, snap: SnapConfig, chosen: Selection) -> EditContext<'_> {
        EditContext {
            doc: self.draft.doc(),
            piece: self.piece,
            nodes: &self.nodes,
            selection: chosen,
            view: View::default(),
            snap,
        }
    }

    /// Where a node sits on the glass.
    pub(super) fn on_glass(&self, node: usize) -> Pos2 {
        View::default().to_screen(self.nodes[node].1)
    }
}

/// The snap put out, so a test moves by exactly what it says.
pub(super) fn free() -> SnapConfig {
    SnapConfig {
        on: false,
        ..SnapConfig::default()
    }
}

/// A distance in centimetres, on the glass.
pub(super) fn glass(cm: f64) -> f32 {
    (cm * View::default().scale()) as f32
}

/// The x a `MovePoint` binds, in centimetres, against the block's own body.
pub(super) fn bound_x(command: &Command, draft: &Draft) -> f64 {
    let Command::MovePoint { to, .. } = command else {
        panic!("a drag frame moves a point: {command:?}");
    };
    to[0].eval(draft.env()).expect("the binding resolves")
}

/// The nodes a selection holds, in key order.
pub(super) fn chosen(feedback: &Feedback) -> Vec<PointKey> {
    feedback
        .select
        .as_ref()
        .expect("the event chose something")
        .points()
        .collect()
}

#[test]
fn a_click_without_movement_does_not_open_a_gesture() {
    let table = table();
    let ctx = table.context(free());
    let at = table.on_glass(1);
    let (gesture, commands, feedback) =
        update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    assert!(commands.is_empty());
    assert_eq!(feedback.stack, Some(Stack::Open(MOVE)));
    assert_eq!(chosen(&feedback), [table.nodes[1].0]);
    let (gesture, commands, feedback) = update(gesture, Input::Up(at, Mods::default()), &ctx);
    assert_eq!(gesture, Gesture::Idle);
    assert!(commands.is_empty(), "a click edits nothing");
    assert_eq!(feedback.stack, Some(Stack::Close));
    assert_eq!(feedback.ask, None);
}

#[test]
fn a_tremor_under_a_hair_is_still_a_click() {
    let table = table();
    let ctx = table.context(free());
    let at = table.on_glass(1);
    let (gesture, _, _) = update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    let (gesture, commands, _) = update(
        gesture,
        Input::Move(at + vec2(1.0, 0.0), Mods::default()),
        &ctx,
    );
    assert!(commands.is_empty());
    let (_, commands, feedback) = update(gesture, Input::Up(at, Mods::default()), &ctx);
    assert!(commands.is_empty());
    assert_eq!(feedback.stack, Some(Stack::Close));
}

#[test]
fn a_whole_drag_emits_one_move_per_frame() {
    let table = table();
    let ctx = table.context(free());
    let at = table.on_glass(0);
    let start = table.nodes[0].1;
    let (mut gesture, _, _) = update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    let mut written = Vec::new();
    for step in 1..=3 {
        let away = vec2(glass(f64::from(step)), 0.0);
        let (next, commands, feedback) =
            update(gesture, Input::Move(at + away, Mods::default()), &ctx);
        gesture = next;
        assert_eq!(commands.len(), 1);
        assert!(feedback.stack.is_none(), "one gesture, one entry");
        written.push(bound_x(&commands[0], &table.draft));
    }
    for (step, x) in written.iter().enumerate() {
        let expected = start[0] + step as f64 + 1.0;
        assert!(
            (x - expected).abs() < 1.0e-9,
            "a free drag still writes tenths: {x} vs {expected}"
        );
    }
    let (_, commands, feedback) = update(gesture, Input::Up(at, Mods::default()), &ctx);
    assert!(commands.is_empty());
    assert_eq!(feedback.stack, Some(Stack::Close), "a literal asks nothing");
}

#[test]
fn dragging_a_coordinate_written_as_a_formula_rewrites_the_formula() {
    let table = table();
    let ctx = table.context(free());
    let at = table.on_glass(1);
    let (gesture, _, _) = update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    let away = vec2(glass(2.0), 0.0);
    let (gesture, commands, _) = update(gesture, Input::Move(at + away, Mods::default()), &ctx);
    let Command::MovePoint { to, .. } = &commands[0] else {
        panic!("a drag frame moves a point");
    };
    assert!(matches!(to[0], Binding::Formula(_)), "{:?}", to[0]);
    assert_eq!(to[0].source(), "cintura / 4 + 3");
    assert!(matches!(to[1], Binding::Literal(_)), "y was a plain zero");

    let (_, commands, feedback) = update(gesture, Input::Up(at + away, Mods::default()), &ctx);
    assert!(commands.is_empty());
    assert_eq!(
        feedback.stack, None,
        "the modal closes the entry, not the up"
    );
    let ask = feedback.ask.expect("a formula was rewritten");
    assert_eq!(ask.rows.len(), 1);
    assert_eq!(ask.rows[0].axis, "X");
    assert_eq!(ask.rows[0].before, "cintura / 4 + 1");
    assert_eq!(ask.rows[0].after, "cintura / 4 + 3");
}

#[test]
fn escape_aborts_without_a_command() {
    let table = table();
    let ctx = table.context(free());
    let at = table.on_glass(1);
    let (gesture, _, _) = update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    let (gesture, _, _) = update(
        gesture,
        Input::Move(at + vec2(glass(2.0), 0.0), Mods::default()),
        &ctx,
    );
    let (gesture, commands, feedback) =
        update(gesture, Input::Key(Key::Escape, Mods::default()), &ctx);
    assert_eq!(gesture, Gesture::Idle);
    assert!(
        commands.is_empty(),
        "the way back is the stack, not an edit"
    );
    assert_eq!(feedback.stack, Some(Stack::Cancel));
}

#[test]
fn the_snap_puts_a_dragged_node_on_the_grid() {
    let table = table();
    let ctx = table.context(SnapConfig::default());
    let at = table.on_glass(0);
    let (gesture, _, _) = update(Gesture::Idle, Input::Down(at, Mods::default()), &ctx);
    let away = vec2(glass(3.4), glass(2.1));
    let (_, commands, feedback) = update(gesture, Input::Move(at + away, Mods::default()), &ctx);
    let snapped = feedback
        .snapped
        .expect("a drag frame reports what caught it");
    assert_eq!(snapped.kind, Some(SnapKind::Grid));
    let Command::MovePoint { to, .. } = &commands[0] else {
        panic!("a drag frame moves a point");
    };
    assert_eq!(to[0], Binding::literal(3.0));
    assert_eq!(to[1], Binding::literal(2.0));
}
