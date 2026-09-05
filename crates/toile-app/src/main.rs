#![allow(missing_docs, reason = "a binary publishes no API surface")]

mod bars;
mod bind;
mod camera;
mod file;
mod glyph;
mod pattern;
mod render;
mod tabs;
mod theme;
mod viewport;
mod widgets;

use std::path::Path;

use eframe::egui;
use eframe::egui_wgpu::RenderState;
use toile_engine::draft::Doc;
use toile_engine::export;
use toile_engine::session::{Session, SessionError};

use crate::file::{Action, File};
use crate::tabs::Tab;
use crate::theme::Theme;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1320.0, 780.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native("Toile", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}

struct App {
    theme: Theme,
    tab: Tab,
    session: Session,
    rs: RenderState,
    file: File,
    patronaje: tabs::patronaje::State,
    probador: tabs::probador::State,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let theme = Theme::sastreria();
        theme.apply(&cc.egui_ctx);
        let session = Session::demo_bodice();
        let rs = cc
            .wgpu_render_state
            .clone()
            .expect("eframe was configured with the wgpu renderer");
        let probador = tabs::probador::State::new(rs.clone(), &theme, &session);
        Self {
            theme,
            tab: Tab::Patronaje,
            session,
            rs,
            file: File::default(),
            patronaje: tabs::patronaje::State::default(),
            probador,
        }
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
    fn act(&mut self, action: Action) {
        match action {
            Action::New => self.start(None),
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
            None => self.install(Session::demo_bodice()),
        }
        let now = self.session.revision();
        self.file.settle(None, now);
    }

    /// Puts a pattern from disk on the table.
    fn open_file(&mut self) {
        if !self.discardable() {
            return;
        }
        let Some(picked) = file::open() else {
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
            file::save_as(self.file.stem())
        } else {
            self.file.path().map(Path::to_path_buf)
        };
        let Some(path) = path else {
            return;
        };
        match file::write(&path, &text) {
            Ok(()) => {
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

/// The file keys, which belong to the program and not to any one tab.
fn shortcut(ui: &egui::Ui) -> Option<Action> {
    ui.input(|i| {
        if !i.modifiers.command {
            return None;
        }
        if i.key_pressed(egui::Key::O) {
            return Some(Action::Open);
        }
        if i.key_pressed(egui::Key::S) {
            return Some(if i.modifiers.shift {
                Action::SaveAs
            } else {
                Action::Save
            });
        }
        None
    })
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let revision = self.session.revision();
        let asked = bars::top(ui, &self.theme, &mut self.tab, &self.file, revision);
        bars::status(ui, &self.theme, self.tab, &self.session, &self.patronaje);
        let mut workspace = tabs::Workspace {
            theme: &self.theme,
            session: &mut self.session,
            patronaje: &mut self.patronaje,
            probador: &mut self.probador,
        };
        self.tab.show(ui, &mut workspace);
        if let Some(action) = self
            .patronaje
            .asked
            .take()
            .or(asked)
            .or_else(|| shortcut(ui))
        {
            self.act(action);
        }
        // The sim advances on its own clock, so a frame is only final once it
        // has both caught up with the last edit and gone back to sleep.
        if !self.session.settled() {
            ui.ctx().request_repaint();
        }
    }
}
