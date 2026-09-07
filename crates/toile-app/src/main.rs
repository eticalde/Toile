#![allow(missing_docs, reason = "a binary publishes no API surface")]

mod bars;
mod bind;
mod config;
mod document;
mod file;
mod glyph;
mod pattern;
mod tabs;
mod theme;
mod viewport;
mod widgets;

use eframe::egui;
use eframe::egui_wgpu::RenderState;
use toile_engine::session::Session;

use crate::file::{Action, File};
use crate::tabs::Tab;
use crate::theme::Theme;

fn main() -> eframe::Result {
    let prefs = config::Prefs::load();
    let viewport = match prefs.window {
        Some([x, y, w, h]) => egui::ViewportBuilder::default()
            .with_position([x, y])
            .with_inner_size([w, h]),
        None => egui::ViewportBuilder::default().with_inner_size([1320.0, 780.0]),
    };
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "Toile",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, prefs)))),
    )
}

struct App {
    theme: Theme,
    tab: Tab,
    session: Session,
    rs: RenderState,
    file: File,
    patronaje: tabs::patronaje::State,
    probador: tabs::probador::State,
    prefs: config::Prefs,
    /// The name being typed for a new product, while its dialog is open.
    new_product: Option<String>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, prefs: config::Prefs) -> Self {
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
            prefs,
            new_product: None,
        }
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
        // Track the window each frame so on_exit writes where it ended up. The
        // read is cheap and the disk write waits for exit.
        if let Some(window) = window_geometry(ui.ctx()) {
            self.prefs.window = Some(window);
        }
        // The mesher answers on its own thread, and this is the once-a-frame
        // collection that lands its rebuilds. It runs before the bars so a
        // refused contour reaches the status bar on this very frame.
        if let Err(why) = self.session.poll_remesh() {
            self.patronaje.refused = Some(why.to_string());
        }
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
        if self.new_product.is_some() {
            let ctx = ui.ctx().clone();
            self.new_product_dialog(&ctx);
        }
        // The sim advances on its own clock, so a frame is only final once it
        // has both caught up with the last edit and gone back to sleep. A
        // rebuild out with the mesher asks for frames on its own: queueing it
        // does not move the generation, so a settled drape would otherwise
        // stop the very frames that collect it.
        if !self.session.settled() || self.session.remeshing() {
            ui.ctx().request_repaint();
        }
    }

    fn on_exit(&mut self) {
        self.prefs.save();
    }
}

/// The window as `[x, y, width, height]`: outer top-left, inner size, the pair
/// [`egui::ViewportBuilder`] restores it from. `None` until the platform has
/// reported both rects.
fn window_geometry(ctx: &egui::Context) -> Option<[f32; 4]> {
    ctx.input(|i| {
        let info = i.viewport();
        let pos = info.outer_rect?.min;
        let size = info.inner_rect?.size();
        Some([pos.x, pos.y, size.x, size.y])
    })
}
