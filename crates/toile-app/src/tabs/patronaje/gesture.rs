use eframe::egui::{Key, Pos2, Rect, Vec2};
use toile_engine::draft::{Axis, Binding, Doc, PieceKey, PointKey};

use super::snap::{SnapConfig, Snapped};
use super::state::Selection;
use super::view::View;
use crate::bind;

/// What the pointer is in the middle of doing.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Gesture {
    /// Nothing: the pointer is only looking.
    #[default]
    Idle,
    /// Sliding the drawing under the pointer.
    Pan {
        /// Where the pointer was when it last slid it.
        from: Pos2,
    },
    /// Drawing a rectangle over the mat to choose what falls inside it.
    Marquee {
        /// The corner the pointer was pressed at, in centimetres.
        from: [f64; 2],
        /// The corner it has reached, in centimetres.
        to: [f64; 2],
    },
    /// Moving the chosen nodes.
    Drag(Box<Drag>),
}

/// The nodes taken in hand, as the gesture holds them.
pub fn holding(drag: Drag) -> Gesture {
    Gesture::Drag(Box::new(drag))
}

/// One node in hand, with what it was bound to when the gesture took hold.
#[derive(Debug, Clone, PartialEq)]
pub struct Held {
    /// The node itself.
    pub point: PointKey,
    /// What the drawing calls it, for the question a release may ask.
    pub name: String,
    /// What its two coordinates were bound to when it was grabbed.
    pub origin: [Binding; 2],
    /// Where it resolved to then, in centimetres.
    pub from: [f64; 2],
}

/// The nodes on their way somewhere, moving together.
///
/// Each carries what it was bound to when it was grabbed, because the document
/// is written on every frame of the drag and the answer to the question the
/// release asks has to come from before the first one. The first is the one
/// the pointer took hold of: the whole gesture is measured from it.
#[derive(Debug, Clone, PartialEq)]
pub struct Drag {
    /// The nodes in hand, the one under the pointer first.
    pub nodes: Vec<Held>,
    /// Where the pointer was pressed, on the glass.
    pub grab: Pos2,
    /// Where the node under the pointer has been taken, in centimetres.
    pub to: [f64; 2],
    /// Whether the pointer ever really left: a click is not a drag.
    pub moved: bool,
    /// The resolution the last frame wrote at, in centimetres.
    pub step: f64,
    /// The exact value being typed, while the precision box is open.
    pub typed: Option<Typed>,
}

/// The precision box: which coordinate it writes, and the text so far.
#[derive(Debug, Clone, PartialEq)]
pub struct Typed {
    /// The coordinate the number lands on.
    pub axis: Axis,
    /// What has been typed into it.
    pub buffer: String,
}

/// The question a release asks when the drag rewrote a formula.
#[derive(Debug, Clone, PartialEq)]
pub struct Ask {
    /// One row per coordinate the drag rewrote.
    pub rows: Vec<AskRow>,
}

/// One coordinate's formula, before and after the drag.
#[derive(Debug, Clone, PartialEq)]
pub struct AskRow {
    /// The coordinate, as the modal names it: the node and the axis when the
    /// gesture holds several nodes, the axis alone when it holds one.
    pub axis: String,
    /// What its author wrote.
    pub before: String,
    /// What the drag made of it.
    pub after: String,
}

/// One thing that happened to the pointer or the keyboard.
#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    /// The primary button went down on the mat.
    Down(Pos2, Mods),
    /// The pointer moved while it was down.
    Move(Pos2, Mods),
    /// The primary button came back up.
    Up(Pos2, Mods),
    /// A key went down.
    Key(Key, Mods),
    /// Characters were typed.
    Text(String),
}

/// The keys held while it happened.
#[expect(
    clippy::struct_excessive_bools,
    reason = "one flag per key the mat reads, which is what a modifier set is"
)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Mods {
    /// Holds the gesture to an axis of its anchor.
    pub shift: bool,
    /// Puts the snap out while it is held.
    pub ctrl: bool,
    /// The platform's own modifier: `Cmd` on macOS, `Ctrl` elsewhere.
    pub command: bool,
    /// Space, held: it turns a drag over the mat into a pan.
    pub space: bool,
}

/// Where the undo stack moves when an event is applied.
///
/// `Open` happens before the commands the same event hands back; the other
/// three happen after them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stack {
    /// Start an entry under this name.
    Open(&'static str),
    /// Close it. One that edited nothing leaves nothing.
    Close,
    /// Take the last entry back.
    Undo,
    /// Take it back and throw it away: what was refused is not redoable.
    Cancel,
    /// Put it back in.
    Redo,
}

/// What one event asks of the surface besides editing the document.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Feedback {
    /// Screen points the drawing slides by.
    pub pan: Vec2,
    /// The selection the event makes, when it makes one.
    pub select: Option<Selection>,
    /// Where the pointer landed and what caught it, while a drag is live.
    pub snapped: Option<Snapped>,
    /// Where the undo stack moves.
    pub stack: Option<Stack>,
    /// The question the release leaves for the modal to put.
    pub ask: Option<Ask>,
}

/// The read-only borrow a gesture is reduced against.
pub struct EditContext<'a> {
    /// The document, for the bindings a drag rewrites.
    pub doc: &'a Doc,
    /// The piece on the table, for the names its nodes go by.
    pub piece: PieceKey,
    /// The piece's nodes, resolved, in contour order, in centimetres.
    pub nodes: &'a [(PointKey, [f64; 2])],
    /// What is chosen right now, which is what a press takes in hand.
    pub selection: Selection,
    /// Where the document lies on the glass.
    pub view: View,
    /// What the pointer catches.
    pub snap: SnapConfig,
}

impl Drag {
    /// The node the pointer took hold of; the gesture is measured from it.
    pub fn anchor(&self) -> &Held {
        self.nodes.first().expect("a drag holds at least one node")
    }

    /// How far the gesture has taken its nodes, in centimetres.
    pub fn delta(&self) -> [f64; 2] {
        let from = self.anchor().from;
        [self.to[0] - from[0], self.to[1] - from[1]]
    }

    /// Where each node is bound now, rounded to `step` centimetres.
    ///
    /// Every node takes the same delta, so a gesture over a whole corner of
    /// the piece keeps its shape. A coordinate written as a formula keeps its
    /// formula: the delta is absorbed into the adjustment term, so the nodes
    /// stay parametric all through the gesture instead of only after it.
    pub fn placed(&self, step: f64) -> Vec<(PointKey, [Binding; 2])> {
        let delta = self.delta();
        self.nodes
            .iter()
            .map(|held| {
                let to = [0, 1].map(|k| {
                    bind::placed(&held.origin[k], held.from[k] + delta[k], delta[k], step)
                });
                (held.point, to)
            })
            .collect()
    }

    /// What the drag makes of every coordinate written as a formula.
    ///
    /// A coordinate whose source comes out unchanged is not a rewrite and
    /// leaves no row, so a drag too small to show asks nothing. `except` is
    /// the coordinate of the anchor the precision box has already written by
    /// hand, which is not the drag's doing and asks nothing either.
    pub fn rewrites(&self, step: f64, except: Option<Axis>) -> Vec<AskRow> {
        let delta = self.delta();
        let many = self.nodes.len() > 1;
        let anchor = self.anchor().point;
        self.nodes
            .iter()
            .flat_map(|held| {
                [(Axis::X, 0), (Axis::Y, 1)]
                    .into_iter()
                    .map(move |pair| (held, pair))
            })
            .filter(|&(held, (axis, _))| !(held.point == anchor && except == Some(axis)))
            .filter_map(|(held, (axis, k))| {
                let Binding::Formula(formula) = &held.origin[k] else {
                    return None;
                };
                let after = formula.nudged_source(delta[k], step);
                (after != formula.source()).then(|| AskRow {
                    axis: if many {
                        format!("{} · {}", held.name, name(axis))
                    } else {
                        name(axis).to_owned()
                    },
                    before: formula.source().to_owned(),
                    after,
                })
            })
            .collect()
    }
}

/// The rectangle a marquee covers on the glass.
pub fn band(view: View, from: [f64; 2], to: [f64; 2]) -> Rect {
    Rect::from_two_pos(view.to_screen(from), view.to_screen(to))
}

/// Whether a node in centimetres falls inside the marquee's two corners.
pub fn inside(from: [f64; 2], to: [f64; 2], at: [f64; 2]) -> bool {
    (0..2).all(|k| {
        let (lo, hi) = (from[k].min(to[k]), from[k].max(to[k]));
        (lo..=hi).contains(&at[k])
    })
}

impl Ask {
    /// The question a release asks, when the drag rewrote a formula at all.
    pub fn of(drag: &Drag, step: f64, except: Option<Axis>) -> Option<Ask> {
        let rows = drag.rewrites(step, except);
        (!rows.is_empty()).then_some(Ask { rows })
    }
}

/// How the panels name a coordinate.
pub fn name(axis: Axis) -> &'static str {
    match axis {
        Axis::X => "X",
        Axis::Y => "Y",
    }
}
