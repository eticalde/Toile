mod ask;

pub use ask::{Ask, AskRow, name};
use eframe::egui::{Key, Pos2, Rect, Vec2};
use toile_engine::draft::{Axis, Binding, Doc, PieceKey, PointKey};

use super::curve::Bend;
use super::snap::{SnapConfig, Snapped};
use super::state::{Selection, Tool};
use super::tract::Tract;
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

/// How a point in hand takes the delta of the gesture.
///
/// `Against` is the whole of the tangent pairing, and it is a delta and not a
/// reflection on purpose: a handle that starts opposite its mate about their
/// node stays opposite it under equal and opposite deltas, while forcing the
/// two collinear would have to overwrite a mate written as a formula with a
/// number the person never typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Follow {
    /// It moves with the pointer.
    Along,
    /// It moves the other way, keeping the tangent through its node straight.
    Against,
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
    /// Which way it takes the gesture's delta.
    pub follow: Follow,
}

/// The nodes on their way somewhere, moving together.
///
/// Each carries what it was bound to when the gesture took hold, because the
/// document is written on every frame of the drag and the answer to the
/// question the release asks has to come from before the first one. The first
/// is the one the pointer took hold of: the whole gesture is measured from it.
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
    /// Whether the tangent pairing has been broken for this gesture.
    ///
    /// It latches: once `Alt` has let a handle off its mate, letting the key
    /// go does not put the mate back where symmetry would have kept it. A
    /// tangent that healed itself on release would undo the very asymmetry the
    /// key was held down to make.
    pub free: bool,
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
    /// Breaks the tangent pairing of the handle in hand.
    pub alt: bool,
    /// The platform's own modifier: `Cmd` on macOS, `Ctrl` elsewhere.
    pub command: bool,
    /// Space, held: it turns a drag over the mat into a pan.
    pub space: bool,
}

/// Where the undo stack moves when an event is applied.
///
/// `Open` happens before the commands the same event hands back; `Once`
/// straddles them; the other three happen after them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stack {
    /// Start an entry under this name.
    Open(&'static str),
    /// Open an entry, take the commands of this same event into it, close it.
    Once(&'static str),
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
    /// The tool the event puts in hand, when it changes it.
    pub tool: Option<Tool>,
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
    /// Its tracts as the drawing paints them, curves flattened.
    pub tracts: &'a [Tract],
    /// Its bent tracts, resolved: every handle of the piece and where it sits.
    pub bends: &'a [Bend],
    /// What is chosen right now, which is what a press takes in hand.
    pub selection: Selection,
    /// The tool in hand.
    pub tool: Tool,
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

    /// Every point in hand, in the order the gesture took them.
    ///
    /// This is what the snap has to leave out of its candidates. A point the
    /// gesture is carrying sits where the last frame left it, so catching one
    /// would make the placement depend on the frame before rather than on the
    /// pointer — and a node dragged with its own handles would be catching
    /// its own tangent.
    pub fn keys(&self) -> Vec<PointKey> {
        self.nodes.iter().map(|held| held.point).collect()
    }

    /// How far the gesture has taken its nodes, in centimetres.
    pub fn delta(&self) -> [f64; 2] {
        let from = self.anchor().from;
        [self.to[0] - from[0], self.to[1] - from[1]]
    }

    /// Every point the gesture is still carrying, with the delta it takes.
    ///
    /// A mate dropped by `Alt` stays in hand — the gesture has to remember it
    /// to keep on ignoring it — but it is carried no further.
    pub fn carried(&self) -> impl Iterator<Item = (&Held, [f64; 2])> {
        let delta = self.delta();
        self.nodes
            .iter()
            .filter(move |held| !(self.free && held.follow == Follow::Against))
            .map(move |held| match held.follow {
                Follow::Along => (held, delta),
                Follow::Against => (held, [-delta[0], -delta[1]]),
            })
    }

    /// Where each point in hand is bound now, rounded to `step` centimetres.
    ///
    /// The nodes take the same delta, so a gesture over a whole corner of the
    /// piece keeps its shape, and a handle's mate takes it reversed. A
    /// coordinate written as a formula keeps its formula: the delta is
    /// absorbed into the adjustment term, so the points stay parametric all
    /// through the gesture instead of only after it.
    pub fn placed(&self, step: f64) -> Vec<(PointKey, [Binding; 2])> {
        self.carried()
            .map(|(held, delta)| {
                let to = [0, 1].map(|k| {
                    bind::placed(&held.origin[k], held.from[k] + delta[k], delta[k], step)
                });
                (held.point, to)
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
