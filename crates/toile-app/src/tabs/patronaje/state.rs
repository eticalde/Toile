mod selection;

pub use selection::Selection;
use toile_engine::draft::{Axis, PieceKey, PointKey, VariableKey};

use super::gesture::{Ask, Gesture};
use super::inspector::Grip;
use super::snap::{SnapConfig, Snapped};
use super::view::View;
use crate::file::Action;

/// The tool in hand: what a press on the mat does to the piece in front.
///
/// Every tool but `Sew` works on the one piece in front; `Sew` works across
/// pieces, so it is held only over the whole product and put down on the way
/// into a piece.
///
/// Only editing tools live here. Drawing a new piece is not a tool but an
/// explicit act — "+ Pieza" in the product tree — so a stray press never grows
/// the product a piece nobody asked for. Only the tools whose phases have
/// arrived are here: a variant nothing can choose would be a promise the tiles
/// do not keep.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tool {
    /// Choose and move what is already drawn.
    #[default]
    Select,
    /// Put a node on the tract under the pointer.
    Point,
    /// Bend a straight tract, and pull the handles of a bent one.
    Curve,
    /// Sew one tract to another, on the whole product where both can be seen.
    Sew,
}

/// How much of the product the mat puts in front of the person.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Scope {
    /// Every piece at once, each where it was placed, there to be arranged.
    #[default]
    Product,
    /// The piece in front alone, in its own coordinates, with every tool.
    Piece,
}

/// A field of the inspector somebody is writing in.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Field {
    /// One coordinate of one node.
    Coordinate(PointKey, Axis),
    /// How finely the tract leaving one node is flattened.
    Samples(PointKey),
    /// One measurement of the body the pattern resolves against.
    Measure(String),
    /// One of the pattern's own quantities.
    Variable(VariableKey),
}

/// The text a field holds while it is being written, before it parses.
///
/// The buffer lives here and not in the document, which is what lets a field
/// paint the fault in what has been typed so far without the geometry ever
/// seeing it.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldEdit {
    /// The field the text belongs to.
    pub of: Field,
    /// What has been typed into it.
    pub buffer: String,
}

/// What the drafting tab remembers between frames.
///
/// None of it belongs to the document: the framing, the selection, the gesture
/// in progress, the field being typed into and the label layer are matters of
/// view, so they are never saved and never undone.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    /// Where the document lies on the glass.
    pub view: View,
    /// Whether the mat shows the whole product or one piece of it.
    ///
    /// A product always opens on the whole of it: that is the one view where
    /// every piece it holds can be seen and reached at once.
    pub scope: Scope,
    /// The piece in front, out of the several a product may hold.
    ///
    /// It is the one the panels edit: the piece a detail shows alone, and the
    /// one lit on the whole product. Chosen in the product tree or on the mat,
    /// and set to a piece the moment it is drawn. `None` only on a product
    /// with no pieces yet, where the sole thing to do is draw the first. A
    /// matter of view, never the document's: which piece is in front of the
    /// person is not something the file remembers.
    pub active: Option<PieceKey>,
    /// The piece being renamed in the product tree, and the name typed so far,
    /// while its row is an open field. A matter of view, like the active piece.
    pub renaming: Option<(PieceKey, String)>,
    /// What the inspector is pointed at.
    pub selection: Selection,
    /// The tool the pointer is holding.
    pub tool: Tool,
    /// Whether the drawing writes the names of the nodes.
    pub labels: bool,
    /// Whether every tract carries its length, and not only the one the
    /// pointer is on.
    pub dimensions: bool,
    /// Frames what the mat shows on the next frame that has something to
    /// frame.
    pub frame: bool,
    /// What the tab asks the application to do with the pattern's file.
    pub asked: Option<Action>,
    /// What the pointer is in the middle of doing.
    pub gesture: Gesture,
    /// What the pointer catches.
    pub snap: SnapConfig,
    /// What it caught last, for as long as the gesture holds it.
    pub caught: Option<Snapped>,
    /// The question the mat is waiting on, while it waits.
    pub ask: Option<Ask>,
    /// The field of the inspector being written in, while one is.
    pub editing: Option<FieldEdit>,
    /// The inspector rail a drag has hold of, while one holds it.
    pub grip: Grip,
    /// What the session last refused to do, while it stands.
    ///
    /// A refused edit leaves the document exactly as it was, and nothing on
    /// the mat marks that anything was asked for. That is the sort of thing a
    /// person has to be told, and the status bar is where.
    pub refused: Option<String>,
}

impl State {
    /// Forgets everything that pointed at the last document.
    ///
    /// A new document brings new keys, so a selection, a gesture or a half
    /// written field held over from the last one would point at nothing. The
    /// next product opens on the whole of it, whatever the last one showed.
    pub fn reset(&mut self) {
        self.scope = Scope::Product;
        self.active = None;
        self.renaming = None;
        self.selection = Selection::None;
        self.gesture = Gesture::Idle;
        self.caught = None;
        self.ask = None;
        self.editing = None;
        self.grip = Grip::default();
        self.refused = None;
        self.frame = true;
    }

    /// Puts one piece alone on the mat, framed, with nothing of it chosen.
    ///
    /// A drawing in progress stays on the mat, now over the piece the person
    /// asked for, so walking away from it leaves them there.
    pub fn open(&mut self, piece: PieceKey) {
        match &mut self.gesture {
            Gesture::Drawing { back_to, .. } => *back_to = Scope::Piece,
            // A tract picked for a seam is a matter of view and holds nothing
            // open, so it is simply let go of.
            Gesture::Sewing(_) => self.gesture = Gesture::Idle,
            _ => {}
        }
        if self.tool == Tool::Sew {
            self.tool = Tool::Select;
        }
        self.active = Some(piece);
        self.scope = Scope::Piece;
        self.choose(Selection::None);
        self.frame = true;
    }

    /// Takes the sewing tool in hand, going back to the whole product for it.
    ///
    /// Asked from a piece it waits for the mat to be free, as the way back to
    /// the product does: a gesture or a question may hold the undo stack
    /// open, and the view left behind would leave that entry open with it.
    pub fn sew(&mut self) {
        let free =
            self.ask.is_none() && matches!(self.gesture, Gesture::Idle | Gesture::Drawing { .. });
        if self.scope == Scope::Piece {
            if !free {
                return;
            }
            self.overview();
        }
        self.tool = Tool::Sew;
    }

    /// Goes back to the whole product, framed.
    ///
    /// A drawing in progress is walked away from, the way Escape leaves it:
    /// nothing of it has reached the document, so there is nothing to unwind.
    pub fn overview(&mut self) {
        if matches!(self.gesture, Gesture::Drawing { .. }) {
            self.gesture = Gesture::Idle;
            self.caught = None;
        }
        self.scope = Scope::Product;
        self.choose(Selection::None);
        self.frame = true;
    }

    /// Chooses `selection`, and lets go of any half-typed text whose row it
    /// no longer shows.
    ///
    /// Only a box drawn on the frame the focus leaves it can confirm what it
    /// holds, and a row whose node, tract or piece is no longer chosen is not
    /// drawn. Kept, its text would come back in the row the next time that
    /// row is shown, and the first click in and out of it would write
    /// something nobody confirmed. So the text goes with the row. `open` and
    /// `overview` choose through here, so a change of scope lets go of the
    /// same rows. A measurement or a variable is on the panel over any scope
    /// and any selection, so its box keeps its text and its usual contract:
    /// whatever takes the focus off it confirms it.
    pub fn choose(&mut self, selection: Selection) {
        self.selection = selection;
        let shown = match self.editing.as_ref().map(|edit| &edit.of) {
            Some(Field::Coordinate(point, _)) => self.selection.only() == Some(*point),
            Some(Field::Samples(node)) => self.selection.edge() == Some(*node),
            Some(Field::Measure(_) | Field::Variable(_)) | None => true,
        };
        if !shown {
            self.editing = None;
        }
    }
}

impl Default for State {
    /// A fresh table: the whole product, nothing chosen, the label layer lit,
    /// the snap on, and waiting to frame whatever it is handed.
    fn default() -> State {
        State {
            view: View::default(),
            scope: Scope::Product,
            active: None,
            renaming: None,
            selection: Selection::None,
            tool: Tool::Select,
            labels: true,
            dimensions: false,
            frame: true,
            asked: None,
            gesture: Gesture::Idle,
            snap: SnapConfig::default(),
            caught: None,
            ask: None,
            editing: None,
            grip: Grip::default(),
            refused: None,
        }
    }
}
