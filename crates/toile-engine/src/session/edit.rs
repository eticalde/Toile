use super::spawn::drape_piece;
use super::{Session, SessionError};
use crate::draft::{Command, PieceKey, Recompile};

impl Session {
    /// Applies an edit and recompiles whatever it touched.
    ///
    /// # Errors
    /// `SessionError` when the session has no document, when the document
    /// refuses the command, or when the edit changes a topology the session
    /// cannot mesh again yet.
    pub fn edit(&mut self, command: Command) -> Result<(), SessionError> {
        let draft = self.draft.as_mut().ok_or(SessionError::NoDocument)?;
        let what = draft.edit(command)?;
        self.revision += 1;
        self.settle(what)
    }

    /// Recompiles after an edit, seeding the product again when the pieces on
    /// the stand changed before falling through to the ordinary shape or
    /// topology path.
    ///
    /// Drawing a piece into the document, and undoing one back out, change
    /// *which* pieces drape rather than their geometry, so they are handled
    /// here rather than in [`Session::recompile`].
    ///
    /// # Errors
    /// The same as [`Session::edit`].
    fn settle(&mut self, what: Recompile) -> Result<(), SessionError> {
        let before = self.pieces();
        self.reconcile();
        // The product was seeded whole just now, so the ordinary recompile has
        // nothing left to add.
        if self.pieces() != before {
            return Ok(());
        }
        self.recompile(what)
    }

    /// Brings the pieces on the stand in line with the document: one the
    /// document no longer holds, or one that has only now become meshable,
    /// sends the whole product back down.
    ///
    /// A piece arriving or leaving moves where every piece after it begins in
    /// the combined state, so the drape cannot simply carry on around it. The
    /// pieces already draping are let go again — the price of the simple thing
    /// here, paid on an action the person took deliberately, and never on a
    /// rebuild or a drag.
    fn reconcile(&mut self) {
        let Some(live) = self.draft.as_ref().map(|draft| draft.doc().piece_keys()) else {
            return;
        };
        let draping = self.pieces();
        let gone = draping.iter().any(|key| !live.contains(key));
        let arrived = live
            .iter()
            .any(|&key| !draping.contains(&key) && self.meshes(key));
        if gone || arrived {
            self.reseed();
        }
    }

    /// Whether a piece the document holds could be meshed right now.
    ///
    /// Asking costs the mesh, and the mesh is thrown away. What is being asked
    /// is only whether the set of pieces on the stand has changed; the seeding
    /// that follows meshes them all over again. A piece being drawn answers no
    /// until it has vertices enough, which is what keeps a draw one gesture.
    fn meshes(&self, piece: PieceKey) -> bool {
        self.draft
            .as_ref()
            .is_some_and(|draft| drape_piece(draft, piece).is_ok())
    }

    /// Opens a gesture: every edit until `end_gesture` is one undo entry.
    ///
    /// A session with no document has no history, so the call is a no-op
    /// rather than an error: the caller is bracketing a drag, not editing.
    pub fn begin_gesture(&mut self, label: &'static str) {
        if let Some(draft) = self.draft.as_mut() {
            draft.begin_gesture(label);
        }
    }

    /// Closes the open gesture. One that edited nothing leaves no entry.
    pub fn end_gesture(&mut self) {
        if let Some(draft) = self.draft.as_mut() {
            draft.end_gesture();
        }
    }

    /// Takes back the last entry and re-drapes whatever it changed.
    ///
    /// # Errors
    /// `SessionError` when the session has no document, when the document
    /// refuses an inverse, or when the step crosses a topology the session
    /// cannot mesh again yet.
    pub fn undo(&mut self) -> Result<(), SessionError> {
        let draft = self.draft.as_mut().ok_or(SessionError::NoDocument)?;
        let what = draft.undo()?;
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
        let draft = self.draft.as_mut().ok_or(SessionError::NoDocument)?;
        let what = draft.cancel_gesture()?;
        self.revision += 1;
        self.settle(what)
    }

    /// Puts the last entry undone back, and re-drapes it.
    ///
    /// # Errors
    /// The same as `undo`.
    pub fn redo(&mut self) -> Result<(), SessionError> {
        let draft = self.draft.as_mut().ok_or(SessionError::NoDocument)?;
        let what = draft.redo()?;
        self.revision += 1;
        self.settle(what)
    }

    /// What undo would take back, named for the status bar.
    pub fn undo_label(&self) -> Option<&str> {
        self.draft.as_ref()?.undo_label()
    }

    /// What redo would put back.
    pub fn redo_label(&self) -> Option<&str> {
        self.draft.as_ref()?.redo_label()
    }

    /// Whether there is anything to take back.
    pub fn can_undo(&self) -> bool {
        self.draft
            .as_ref()
            .is_some_and(|draft| draft.undo_depth() > 0)
    }

    /// Whether there is anything to put back.
    pub fn can_redo(&self) -> bool {
        self.draft
            .as_ref()
            .is_some_and(|draft| draft.redo_depth() > 0)
    }

    /// Pays whatever the last change to the document cost the drape.
    ///
    /// The two branches are the two budgets: a shape edit is derived here and
    /// now, a topology edit goes to the mesher and comes back when it is
    /// ready. Both name the pieces they touched, and a piece nobody named is
    /// not read at all. Neither of them blocks on the solver.
    ///
    /// # Errors
    /// The same as [`Session::edit`].
    fn recompile(&mut self, what: Recompile) -> Result<(), SessionError> {
        match what {
            Recompile::Shape(pieces) => self.rederive(&pieces),
            Recompile::Topology(pieces) => self.remesh(&pieces),
            Recompile::Nothing => Ok(()),
        }
    }
}
