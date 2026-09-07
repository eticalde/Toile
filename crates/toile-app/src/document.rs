use std::path::Path;

use eframe::egui;
use toile_engine::draft::Doc;
use toile_engine::export;
use toile_engine::session::{Session, SessionError};

use crate::file::{self, Action, File};
use crate::tabs;

impl crate::App {
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

    /// Puts a fresh blank product on the table under the name it was given.
    fn create_named(&mut self, title: String) {
        self.install(Session::blank());
        let now = self.session.revision();
        self.file = File::new_named(title);
        self.file.settle(None, now);
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
        let Some(picked) = file::open(self.prefs.last_dir.as_deref()) else {
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
            file::save_as(self.file.stem(), self.prefs.last_dir.as_deref())
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

    /// Whether work nobody has written down may be thrown away, which is only
    /// ever the person's own answer.
    fn discardable(&self) -> bool {
        !self.file.dirty(self.session.revision()) || file::confirm_discard(self.file.name())
    }
}
