use eframe::egui::{Pos2, vec2};
use toile_engine::draft::{Binding, Doc, Draft, PieceKey, PointKey, block};

use super::*;
use crate::tabs::patronaje::curve::{self, Bend};
use crate::tabs::patronaje::inner::{Drawn, Tick};
use crate::tabs::patronaje::snap::{SnapConfig, SnapKind};
use crate::tabs::patronaje::tract::{self, Tract};
use crate::tabs::patronaje::view::View;

/// The block on the table, with its contour already resolved.
pub(in crate::tabs::patronaje) struct Table {
    pub(in crate::tabs::patronaje) draft: Draft,
    pub(in crate::tabs::patronaje) piece: PieceKey,
    pub(in crate::tabs::patronaje) nodes: Vec<(PointKey, [f64; 2])>,
    pub(in crate::tabs::patronaje) tracts: Vec<Tract>,
    bends: Vec<Bend>,
    lines: Vec<Drawn>,
    ticks: Vec<Tick>,
}

pub(in crate::tabs::patronaje) fn table() -> Table {
    table_of(block::trouser_front())
}

/// Any product on the table, worked out the way one frame of the mat works it
/// out: the piece in front is the first it holds.
pub(in crate::tabs::patronaje) fn table_of(doc: Doc) -> Table {
    let draft = Draft::from_doc(doc).expect("the product resolves");
    let piece = draft.doc().piece_keys()[0];
    let tracts = tract::of(&draft, piece);
    Table {
        nodes: draft.points_cm(piece).to_vec(),
        bends: curve::bends(&draft, piece),
        lines: inner::of(&draft, piece, &tracts, [0.0, 0.0]),
        ticks: inner::ticks(&draft, piece, &tracts),
        tracts,
        draft,
        piece,
    }
}

impl Table {
    /// The context a gesture reduces against, with the snap in whatever state
    /// the test needs it and nothing chosen.
    pub(in crate::tabs::patronaje) fn context(&self, snap: SnapConfig) -> EditContext<'_> {
        self.holding(snap, Selection::None)
    }

    /// The same, with `chosen` already in hand.
    pub(in crate::tabs::patronaje) fn holding(
        &self,
        snap: SnapConfig,
        chosen: Selection,
    ) -> EditContext<'_> {
        EditContext {
            doc: self.draft.doc(),
            piece: self.piece,
            nodes: &self.nodes,
            tracts: &self.tracts,
            bends: &self.bends,
            lines: &self.lines,
            ticks: &self.ticks,
            selection: chosen,
            tool: Tool::Select,
            view: View::default(),
            snap,
        }
    }

    /// The same context with `tool` in hand and the snap live, which is what a
    /// tool's own reducer is asked against: the ladder is how the Line tool
    /// tells a place on the contour from one on the bare cloth.
    pub(in crate::tabs::patronaje) fn wielding(&self, tool: Tool) -> EditContext<'_> {
        EditContext {
            tool,
            ..self.context(SnapConfig::default())
        }
    }

    /// The lines the mat drew for the piece in front.
    pub(in crate::tabs::patronaje) fn drawn(&self) -> &[Drawn] {
        &self.lines
    }

    /// The notches the mat drew on it.
    pub(in crate::tabs::patronaje) fn ticks(&self) -> &[Tick] {
        &self.ticks
    }

    /// One tract of the piece that bends, when it draws any.
    pub(in crate::tabs::patronaje) fn bent(&self) -> Option<&Bend> {
        self.bends.first()
    }

    /// Where a node sits on the glass.
    pub(in crate::tabs::patronaje) fn on_glass(&self, node: usize) -> Pos2 {
        View::default().to_screen(self.nodes[node].1)
    }
}

/// The snap put out, so a test moves by exactly what it says.
pub(in crate::tabs::patronaje) fn free() -> SnapConfig {
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
    // The waist opens the hip curve, so its tangent came along, and the
    // question covers the formula on the handle as well as the one on the
    // node. The y of the handle is a formula too and asks nothing: the drag
    // was level, and a delta of nothing rewrites nothing.
    let ask = feedback.ask.expect("a formula was rewritten");
    assert_eq!(ask.rows.len(), 2, "{ask:?}");
    assert_eq!(ask.rows[0].axis, "cintura_lat · X");
    assert_eq!(ask.rows[0].before, "cintura / 4 + 1");
    assert_eq!(ask.rows[0].after, "cintura / 4 + 3");
    assert_eq!(ask.rows[1].axis, "manija_cadera_1 · X");
    assert_eq!(ask.rows[1].after, "cintura / 4 + 3");
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
