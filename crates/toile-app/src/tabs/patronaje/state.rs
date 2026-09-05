use std::collections::BTreeSet;

use toile_engine::draft::{Axis, PointKey, VariableKey};

use super::gesture::{Ask, Gesture};
use super::snap::{SnapConfig, Snapped};
use super::view::View;
use crate::file::Action;

/// What the inspector is pointed at.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Selection {
    /// The piece as a whole.
    #[default]
    None,
    /// Nodes of it, in key order.
    Points(BTreeSet<PointKey>),
    /// The tract leaving one node.
    Edge(PointKey),
}

impl Selection {
    /// The selection one node on its own makes.
    pub fn point(key: PointKey) -> Selection {
        Selection::Points(BTreeSet::from([key]))
    }

    /// The set of nodes chosen, when nodes are what is chosen.
    pub fn chosen(&self) -> Option<&BTreeSet<PointKey>> {
        match self {
            Selection::Points(keys) => Some(keys),
            Selection::None | Selection::Edge(_) => None,
        }
    }

    /// The nodes chosen, in key order; nothing when none are.
    pub fn points(&self) -> impl Iterator<Item = PointKey> + '_ {
        self.chosen().into_iter().flatten().copied()
    }

    /// How many nodes are chosen.
    pub fn count(&self) -> usize {
        self.chosen().map_or(0, BTreeSet::len)
    }

    /// The one node chosen, when exactly one is.
    pub fn only(&self) -> Option<PointKey> {
        let keys = self.chosen()?;
        match keys.len() {
            1 => keys.first().copied(),
            _ => None,
        }
    }

    /// Whether `key` is one of the nodes chosen.
    pub fn holds(&self, key: PointKey) -> bool {
        self.chosen().is_some_and(|keys| keys.contains(&key))
    }

    /// The node the chosen tract leaves, when a tract is chosen.
    pub fn edge(&self) -> Option<PointKey> {
        match self {
            Selection::Edge(key) => Some(*key),
            Selection::None | Selection::Points(_) => None,
        }
    }
}

/// The tool in hand, out of the nine the panel offers.
///
/// Only the ones whose phases have arrived are here: a variant nothing can
/// choose would be a promise the tiles do not keep.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tool {
    /// Choose and move what is already drawn.
    #[default]
    Select,
    /// Put a node on the tract under the pointer.
    Point,
    /// Draw a new piece, vertex by vertex.
    Line,
    /// Bend a straight tract, and pull the handles of a bent one.
    Curve,
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
    /// What the inspector is pointed at.
    pub selection: Selection,
    /// The tool the pointer is holding.
    pub tool: Tool,
    /// Whether the drawing writes the names of the nodes.
    pub labels: bool,
    /// Whether every tract carries its length, and not only the one the
    /// pointer is on.
    pub dimensions: bool,
    /// Frames the piece on the next frame that has one to frame.
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
    /// written field held over from the last one would point at nothing.
    pub fn reset(&mut self) {
        self.selection = Selection::None;
        self.gesture = Gesture::Idle;
        self.caught = None;
        self.ask = None;
        self.editing = None;
        self.refused = None;
        self.frame = true;
    }
}

impl Default for State {
    /// A fresh table: nothing chosen, the label layer lit, the snap on, and
    /// waiting to frame whatever it is handed.
    fn default() -> State {
        State {
            view: View::default(),
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
            refused: None,
        }
    }
}
