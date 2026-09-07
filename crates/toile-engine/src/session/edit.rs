use super::{Session, SessionError};
use crate::draft::{Command, Recompile};

impl Session {
    /// Applies an edit and recompiles whatever it touched.
    ///
    /// # Errors
    /// `SessionError` when the session has no document, when the document
    /// refuses the command, or when the edit changes a topology the session
    /// cannot mesh again yet.
    pub fn edit(&mut self, command: Command) -> Result<(), SessionError> {
        let drafted = self.drafted.as_mut().ok_or(SessionError::NoDocument)?;
        let what = drafted.draft.edit(command)?;
        self.revision += 1;
        self.settle(what)
    }

    /// Recompiles after an edit, adopting a first piece or dropping a removed
    /// one before falling through to the ordinary shape or topology path.
    ///
    /// Drawing the first piece into a blank document, and undoing back to
    /// blank, both change *which* piece drapes rather than its geometry, so
    /// they are handled here rather than in [`Session::recompile`].
    ///
    /// # Errors
    /// The same as [`Session::edit`].
    fn settle(&mut self, what: Recompile) -> Result<(), SessionError> {
        let before = self.piece();
        self.reconcile();
        // A piece was just adopted or dropped: its drape was seeded or torn
        // down whole, so the ordinary recompile has nothing to add.
        if self.piece() != before {
            return Ok(());
        }
        self.recompile(what)
    }

    /// Brings the draping piece in line with the document: drop one the
    /// document no longer holds, then adopt the first it does when none drapes.
    ///
    /// Adopting is best-effort. A piece being drawn lands empty and gains its
    /// vertices one command at a time, so it cannot mesh until it has enough of
    /// them; a failure here simply leaves the table blank until the next
    /// vertex, which is what keeps the whole draw one smooth gesture.
    fn reconcile(&mut self) {
        if let (Some(piece), Some(drafted)) = (self.piece(), self.drafted.as_ref())
            && !drafted.draft.doc().piece_keys().contains(&piece)
        {
            self.unseed();
        }
        if self.piece().is_none()
            && let Some(drafted) = self.drafted.as_ref()
            && let Some(&piece) = drafted.draft.doc().piece_keys().first()
        {
            let _ = self.seed_piece(piece);
        }
    }

    /// Opens a gesture: every edit until `end_gesture` is one undo entry.
    ///
    /// A session with no document has no history, so the call is a no-op
    /// rather than an error: the caller is bracketing a drag, not editing.
    pub fn begin_gesture(&mut self, label: &'static str) {
        if let Some(drafted) = self.drafted.as_mut() {
            drafted.draft.begin_gesture(label);
        }
    }

    /// Closes the open gesture. One that edited nothing leaves no entry.
    pub fn end_gesture(&mut self) {
        if let Some(drafted) = self.drafted.as_mut() {
            drafted.draft.end_gesture();
        }
    }

    /// Takes back the last entry and re-drapes whatever it changed.
    ///
    /// # Errors
    /// `SessionError` when the session has no document, when the document
    /// refuses an inverse, or when the step crosses a topology the session
    /// cannot mesh again yet.
    pub fn undo(&mut self) -> Result<(), SessionError> {
        let drafted = self.drafted.as_mut().ok_or(SessionError::NoDocument)?;
        let what = drafted.draft.undo()?;
        self.revision += 1;
        self.settle(what)
    }

    /// Drops the open gesture and re-drapes whatever it had changed.
    ///
    /// The refused edits leave nothing for redo: this is the way out of a
    /// gesture, not a step through the history.
    ///
    /// # Errors
    /// The same as `undo`.
    pub fn cancel_gesture(&mut self) -> Result<(), SessionError> {
        let drafted = self.drafted.as_mut().ok_or(SessionError::NoDocument)?;
        let what = drafted.draft.cancel_gesture()?;
        self.revision += 1;
        self.settle(what)
    }

    /// Puts the last entry undone back, and re-drapes it.
    ///
    /// # Errors
    /// The same as `undo`.
    pub fn redo(&mut self) -> Result<(), SessionError> {
        let drafted = self.drafted.as_mut().ok_or(SessionError::NoDocument)?;
        let what = drafted.draft.redo()?;
        self.revision += 1;
        self.settle(what)
    }

    /// What undo would take back, named for the status bar.
    pub fn undo_label(&self) -> Option<&str> {
        self.drafted.as_ref()?.draft.undo_label()
    }

    /// What redo would put back.
    pub fn redo_label(&self) -> Option<&str> {
        self.drafted.as_ref()?.draft.redo_label()
    }

    /// Whether there is anything to take back.
    pub fn can_undo(&self) -> bool {
        self.drafted
            .as_ref()
            .is_some_and(|held| held.draft.undo_depth() > 0)
    }

    /// Whether there is anything to put back.
    pub fn can_redo(&self) -> bool {
        self.drafted
            .as_ref()
            .is_some_and(|held| held.draft.redo_depth() > 0)
    }

    /// Pays whatever the last change to the document cost the drape.
    ///
    /// The two branches are the two budgets: a shape edit is derived here and
    /// now, a topology edit goes to the mesher and comes back when it is
    /// ready. Neither of them blocks on the solver.
    fn recompile(&mut self, what: Recompile) -> Result<(), SessionError> {
        let Some(piece) = self.piece() else {
            return Ok(());
        };
        match what {
            Recompile::Shape(pieces) if pieces.contains(&piece) => self.rederive(),
            Recompile::Topology(pieces) if pieces.contains(&piece) => {
                self.remesh();
                Ok(())
            }
            Recompile::Nothing | Recompile::Shape(_) | Recompile::Topology(_) => Ok(()),
        }
    }
}
