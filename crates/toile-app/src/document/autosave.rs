use std::path::Path;
use std::time::{Duration, Instant};

use eframe::egui;

use crate::file;

/// How long the document sits still before an autosave writes it: long enough
/// that a burst of edits coalesces into one write, short enough to keep work.
const AUTOSAVE_IDLE: Duration = Duration::from_millis(800);

impl crate::App {
    /// Writes the document back to its file once it has sat still long enough,
    /// so a placed product keeps itself. Every edit restarts the clock and asks
    /// for a frame, so a burst coalesces into one write that lands even when
    /// the app is otherwise idle; only a product with a home autosaves.
    pub(crate) fn autosave(&mut self, ctx: &egui::Context) {
        let revision = self.session.revision();
        if self.file.path().is_none() || !self.file.dirty(revision) {
            self.autosave_due = None;
            return;
        }
        if revision != self.autosave_rev {
            self.autosave_rev = revision;
            self.autosave_due = Some(Instant::now() + AUTOSAVE_IDLE);
        }
        let Some(due) = self.autosave_due else {
            return;
        };
        if let Some(left) = due.checked_duration_since(Instant::now()) {
            ctx.request_repaint_after(left);
        } else {
            self.autosave_due = None;
            self.autosave_now();
        }
    }

    /// Writes the document back where it lives, quietly: the dirty marker
    /// clearing is the whole of the report, and a fault raises a notice.
    pub(super) fn autosave_now(&mut self) {
        let revision = self.session.revision();
        let Some(path) = self.file.path().map(Path::to_path_buf) else {
            return;
        };
        let Some(text) = self
            .session
            .draft()
            .map(|held| held.doc().to_canonical_json())
        else {
            return;
        };
        match file::write(&path, &text) {
            Ok(()) => self.file.settle(Some(path), revision),
            Err(why) => self.file.warn(why, revision),
        }
    }
}
