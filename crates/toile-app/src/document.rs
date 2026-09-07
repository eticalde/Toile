use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui;
use toile_engine::draft::Doc;
use toile_engine::export;
use toile_engine::session::{Session, SessionError};

use crate::file::{self, Action, File};
use crate::{config, tabs};

/// How long the document sits still before an autosave writes it: long enough
/// that a burst of edits coalesces into one write, short enough to keep work.
const AUTOSAVE_IDLE: Duration = Duration::from_millis(800);

impl crate::App {
    /// Where a file dialog should open: the last folder used, else the default
    /// patterns folder, which is made if it is not there yet so it can be
    /// shown.
    fn start_dir(&self) -> Option<PathBuf> {
        if let Some(dir) = &self.prefs.last_dir {
            return Some(dir.clone());
        }
        let dir = config::patterns_dir()?;
        let _ = std::fs::create_dir_all(&dir);
        Some(dir)
    }

    /// Puts a document on the table.
    ///
    /// # Errors
    /// `SessionError` when the document does not drape, in which case the
    /// table is left exactly as it was.
    fn open(&mut self, doc: Doc) -> Result<(), SessionError> {
        let session = Session::from_doc(doc)?;
        self.install(session);
        Ok(())
    }

    /// Puts a session on the table.
    ///
    /// A session is a new mesh, which the viewer was not sized for, so its GPU
    /// side is rebuilt around the one that came out; and it is a new set of
    /// keys, so everything the drafting tab was pointing at is forgotten.
    fn install(&mut self, session: Session) {
        self.session = session;
        self.probador = tabs::probador::State::new(self.rs.clone(), &self.theme, &self.session);
        self.patronaje.reset();
    }

    /// Does what the interface asked of the file the pattern lives in.
    pub(crate) fn act(&mut self, action: Action) {
        match action {
            // Ask the name first; the table is cleared only once it is given.
            Action::New if self.discardable() => self.new_product = Some(String::new()),
            Action::New => {}
            Action::Example => self.start(Some(File::example())),
            Action::Open => self.open_file(),
            Action::Save => {
                let unplaced = self.file.path().is_none();
                self.save(unplaced);
            }
            Action::SaveAs => self.save(true),
            Action::Svg => self.export(),
        }
    }

    /// Puts a fresh blank product on the table and gives it a home on disk at
    /// once, so there is nothing to remember to save. When it cannot be
    /// written, it opens under its name in memory until saved by hand, and
    /// the bar says so.
    fn create_named(&mut self, title: String) {
        self.install(Session::blank());
        let now = self.session.revision();
        if let Some(path) = self.place_new(&title) {
            self.prefs.remember(&path);
            self.prefs.save();
            self.file.settle(Some(path), now);
        } else {
            self.file = File::new_named(title);
            self.file.settle(None, now);
            self.file
                .warn("no se pudo crear el archivo; usa Guardar", now);
        }
    }

    /// Writes the blank product now on the table into a fresh file, and hands
    /// back where it went. `None` when it cannot be written.
    fn place_new(&self, title: &str) -> Option<PathBuf> {
        let dir = config::patterns_dir()?;
        std::fs::create_dir_all(&dir).ok()?;
        let path = file::unique_path(&dir, title);
        let text = self.session.draft()?.doc().to_canonical_json();
        file::write(&path, &text).ok()?;
        Some(path)
    }

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
    fn autosave_now(&mut self) {
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

    /// The dialog that names a new product, shown while `new_product` is set.
    ///
    /// Enter or "Crear" makes the product; Escape or "Cancelar" walks away. An
    /// empty name cannot make one: a product is named, even before it is saved.
    pub(crate) fn new_product_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut name) = self.new_product.take() else {
            return;
        };
        let mut decided = None;
        egui::Modal::new(egui::Id::new("nuevo-producto")).show(ctx, |ui| {
            ui.set_width(320.0);
            ui.heading("Nuevo producto");
            ui.add_space(8.0);
            ui.label("Nombre del producto");
            let field = ui.text_edit_singleline(&mut name);
            field.request_focus();
            ui.add_space(12.0);
            let ready = !name.trim().is_empty();
            if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && ready {
                decided = Some(true);
            }
            ui.horizontal(|ui| {
                if ui.button("Cancelar").clicked() {
                    decided = Some(false);
                }
                if ui.add_enabled(ready, egui::Button::new("Crear")).clicked() {
                    decided = Some(true);
                }
            });
        });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            decided = Some(false);
        }
        match decided {
            Some(true) => self.create_named(name.trim().to_owned()),
            Some(false) => {}
            None => self.new_product = Some(name),
        }
    }

    /// Clears the table, and puts a document on it when there is one.
    fn start(&mut self, doc: Option<Doc>) {
        if !self.discardable() {
            return;
        }
        let revision = self.session.revision();
        match doc {
            Some(doc) => {
                if let Err(why) = self.open(doc) {
                    self.file.warn(format!("no se pudo abrir: {why}"), revision);
                    return;
                }
            }
            None => self.install(Session::blank()),
        }
        let now = self.session.revision();
        self.file.settle(None, now);
    }

    /// Puts a pattern from disk on the table.
    fn open_file(&mut self) {
        if !self.discardable() {
            return;
        }
        let Some(picked) = file::open(self.start_dir().as_deref()) else {
            return;
        };
        let revision = self.session.revision();
        match picked.and_then(|(path, doc)| {
            self.open(doc)
                .map(|()| path)
                .map_err(|why| format!("el patrón no se pudo poner sobre la mesa: {why}"))
        }) {
            Ok(path) => {
                let now = self.session.revision();
                self.prefs.remember(&path);
                self.prefs.save();
                self.file.settle(Some(path), now);
            }
            Err(why) => self.file.warn(why, revision),
        }
    }

    /// Writes the pattern back where it came from, or wherever is asked for.
    fn save(&mut self, ask: bool) {
        let revision = self.session.revision();
        let written = self
            .session
            .draft()
            .map(|held| held.doc().to_canonical_json());
        let Some(text) = written else {
            self.file.warn("no hay ningún patrón que guardar", revision);
            return;
        };
        let path = if ask {
            file::save_as(self.file.stem(), self.start_dir().as_deref())
        } else {
            self.file.path().map(Path::to_path_buf)
        };
        let Some(path) = path else {
            return;
        };
        match file::write(&path, &text) {
            Ok(()) => {
                self.prefs.remember(&path);
                self.prefs.save();
                self.file.settle(Some(path), revision);
                let name = self.file.name().to_owned();
                self.file.say(format!("guardado · {name}"), revision);
            }
            Err(why) => self.file.warn(why, revision),
        }
    }

    /// Draws the pattern into an SVG at true scale.
    fn export(&mut self) {
        let revision = self.session.revision();
        let drawn = self.session.draft().map(export::to_svg);
        let text = match drawn {
            Some(Ok(text)) => text,
            Some(Err(why)) => {
                self.file
                    .warn(format!("no se pudo dibujar: {why}"), revision);
                return;
            }
            None => {
                self.file
                    .warn("no hay ningún patrón que exportar", revision);
                return;
            }
        };
        let Some(path) = file::svg_target(self.file.stem()) else {
            return;
        };
        match file::write(&path, &text) {
            Ok(()) => self.file.say("SVG exportado a escala real", revision),
            Err(why) => self.file.warn(why, revision),
        }
    }

    /// Whether work nobody has written down may be thrown away.
    ///
    /// A placed product autosaves, so a pending write is flushed first: leaving
    /// one product for another never asks about changes autosave already keeps.
    /// Only an unplaced product — the edited example, or a creation that could
    /// not be written — can still have unsaved work, and that is the person's
    /// own answer.
    fn discardable(&mut self) -> bool {
        if self.file.path().is_some() && self.file.dirty(self.session.revision()) {
            self.autosave_now();
        }
        !self.file.dirty(self.session.revision()) || file::confirm_discard(self.file.name())
    }
}
