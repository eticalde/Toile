mod ask;
/// The nodes a drag has in hand, and what the pointer's delta does to them.
mod held;

pub use ask::{Ask, AskRow, name};
use eframe::egui::{Key, Pos2, Rect, Vec2};
pub use held::{Drag, Follow, Held, holding};
use toile_engine::draft::{Doc, PieceKey, PointKey};

use super::arrange::Arrange;
use super::curve::Bend;
use super::dart::Darting;
use super::inner::{Drawn, Tick};
/// The precision box lives with the box that paints it; a drag carries one.
pub use super::precision::Typed;
use super::sew::Sewing;
use super::slide::Slide;
use super::snap::{SnapConfig, Snapped};
use super::state::{Scope, Selection, Tool};
use super::trace::Tracing;
use super::tract::Tract;
use super::view::View;

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
    /// Drawing a new piece, vertex by vertex.
    ///
    /// The piece itself is not here because it does not exist yet: no command
    /// goes out until the contour closes, which is what lets Escape abort the
    /// whole drawing without an entry to unwind.
    Drawing {
        /// The vertices already placed, in the order they were clicked, in
        /// centimetres.
        pending: Vec<[f64; 2]>,
        /// Where the next vertex would land, snap and all, in centimetres.
        rubber: [f64; 2],
        /// The scope the drawing was started from, which is where walking
        /// away from it goes back to.
        back_to: Scope,
    },
    /// Moving a whole piece across the product.
    Arrange(Arrange),
    /// One tract picked for a seam, waiting for the tract it is sewn to.
    Sewing(Sewing),
    /// Drawing a line inside a piece, place by place.
    Tracing(Tracing),
    /// Cutting a dart: the legs pressed so far, waiting for the apex.
    Darting(Darting),
    /// Sliding a notch along the tract it was cut into.
    Sliding(Slide),
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
    /// Whether the event asks to leave the piece for the whole product.
    pub overview: bool,
    /// What the event would not do, in the words the person reads.
    ///
    /// Said here and never as an error the document hands back: the panels are
    /// in Spanish and a `DraftError` is not, and a press the mat itself refuses
    /// never reaches the document to be refused there.
    pub refused: Option<&'static str>,
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
    /// The lines drawn inside it, where the mat draws them.
    pub lines: &'a [Drawn],
    /// The notches cut into its contour, where the mat draws them.
    pub ticks: &'a [Tick],
    /// What is chosen right now, which is what a press takes in hand.
    pub selection: Selection,
    /// The tool in hand.
    pub tool: Tool,
    /// Where the document lies on the glass.
    pub view: View,
    /// What the pointer catches.
    pub snap: SnapConfig,
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
