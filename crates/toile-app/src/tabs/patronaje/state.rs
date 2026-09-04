use toile_engine::draft::{Doc, PointKey};

use super::view::View;

/// What the drafting tab remembers between frames.
///
/// None of it belongs to the document: the framing, the selection and the
/// label layer are matters of view, so they are never saved and never undone.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    /// Where the document lies on the glass.
    pub view: View,
    /// The node the inspector is pointed at.
    pub selection: Option<PointKey>,
    /// Whether the drawing writes the names of the nodes.
    pub labels: bool,
    /// Frames the piece on the next frame that has one to frame.
    pub frame: bool,
    /// A document the tab asks the application to open in its place.
    pub pending: Option<Doc>,
}

impl Default for State {
    /// A fresh table: nothing chosen, the label layer lit, waiting to frame
    /// whatever it is handed.
    fn default() -> State {
        State {
            view: View::default(),
            selection: None,
            labels: true,
            frame: true,
            pending: None,
        }
    }
}
