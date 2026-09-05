use toile_doc::PieceKey;

use super::{Draft, DraftError, Recompile};

impl Draft {
    /// Opens a gesture: everything edited until `end_gesture` is one entry.
    pub fn begin_gesture(&mut self, label: &'static str) {
        self.history.begin(label);
    }

    /// Closes the open gesture. One that edited nothing leaves no entry.
    pub fn end_gesture(&mut self) {
        self.history.end();
    }

    /// Takes back the last entry and says what has to be recompiled.
    ///
    /// # Errors
    /// `DraftError` when the document refuses an inverse, or when the state it
    /// goes back to no longer resolves.
    pub fn undo(&mut self) -> Result<Recompile, DraftError> {
        let touched = self.history.undo(&mut self.doc)?;
        self.recover(touched)
    }

    /// Drops the open gesture and says what has to be recompiled.
    ///
    /// This is how a gesture is refused rather than stepped back through: what
    /// the user declined leaves nothing for redo to bring back.
    ///
    /// # Errors
    /// The same as `undo`.
    pub fn cancel_gesture(&mut self) -> Result<Recompile, DraftError> {
        let touched = self.history.cancel(&mut self.doc)?;
        self.recover(touched)
    }

    /// Puts the last entry undone back, and says what has to be recompiled.
    ///
    /// # Errors
    /// The same as `undo`.
    pub fn redo(&mut self) -> Result<Recompile, DraftError> {
        let touched = self.history.redo(&mut self.doc)?;
        self.recover(touched)
    }

    /// What undo would take back, for the status bar.
    pub fn undo_label(&self) -> Option<&str> {
        self.history.undo_label()
    }

    /// What redo would put back.
    pub fn redo_label(&self) -> Option<&str> {
        self.history.redo_label()
    }

    /// How many entries undo can take back, an open gesture included.
    pub fn undo_depth(&self) -> usize {
        self.history.depth()
    }

    /// How many entries redo can put back.
    pub fn redo_depth(&self) -> usize {
        self.history.redo_depth()
    }

    /// Re-resolves after the history moved, and prices what the move cost.
    ///
    /// The class is read off the geometry rather than remembered: a step that
    /// changed how long the flattened contour is has to be meshed again,
    /// whatever the commands inside the entry were.
    fn recover(&mut self, touched: Vec<PieceKey>) -> Result<Recompile, DraftError> {
        let before: Vec<usize> = touched
            .iter()
            .map(|&piece| self.outline(piece).len())
            .collect();
        self.resolve_all()?;
        let remeshed = touched
            .iter()
            .zip(&before)
            .any(|(&piece, &was)| self.outline(piece).len() != was);
        if !remeshed {
            return Ok(Recompile::Shape(touched));
        }
        for &piece in &touched {
            self.pieces.entry(piece).or_default().topology += 1;
        }
        Ok(Recompile::Topology(touched))
    }
}
