#![allow(missing_docs, reason = "a binary publishes no API surface")]

mod camera;
mod glyph;
mod pattern;
mod render;
mod tabs;
mod theme;
mod viewport;
mod widgets;

use eframe::egui;
use eframe::egui_wgpu::RenderState;
use toile_engine::draft::Doc;
use toile_engine::session::Session;

use crate::tabs::Tab;
use crate::theme::Theme;

/// Height of the tab bar, in points.
const TOPBAR_H: f32 = 40.0;
/// Height of the status bar, in points.
const STATUS_H: f32 = 26.0;
/// Inset of the first item in either bar, in points.
const BAR_INSET: i8 = 16;

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
            patronaje: tabs::patronaje::State::default(),
            probador,
        }
    }

    /// Puts a document on the table.
    ///
    /// A document is a new session and therefore a new mesh, which the viewer
    /// was not sized for, so its GPU side is rebuilt around the one that came
    /// out. A document that does not drape leaves the table as it was.
    fn open(&mut self, doc: Doc) {
        let Ok(session) = Session::from_doc(doc) else {
            return;
        };
        self.session = session;
        self.probador = tabs::probador::State::new(self.rs.clone(), &self.theme, &self.session);
        self.patronaje.selection = None;
        self.patronaje.frame = true;
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        top_bar(ui, &self.theme, &mut self.tab);
        status_bar(ui, &self.theme, self.tab, &self.session);
        let mut workspace = tabs::Workspace {
            theme: &self.theme,
            session: &mut self.session,
            patronaje: &mut self.patronaje,
            probador: &mut self.probador,
        };
        self.tab.show(ui, &mut workspace);
        if let Some(doc) = self.patronaje.pending.take() {
            self.open(doc);
        }
        // The sim advances on its own clock, so a frame is only final once it
        // has both caught up with the last edit and gone back to sleep.
        if !self.session.settled() {
            ui.ctx().request_repaint();
        }
    }
}

// ── bars ──────────────────────────────────────────────────────────────────

/// Both bars fill themselves, so egui's own separator line is off: it would
/// reserve a point of the bar's height and then paint over the tab underline
/// that lands in it.
fn bar_frame(theme: &Theme) -> egui::Frame {
    egui::Frame::new()
        .fill(theme.panel)
        .inner_margin(egui::Margin::symmetric(BAR_INSET, 0))
}

fn top_bar(ui: &mut egui::Ui, theme: &Theme, tab: &mut Tab) {
    egui::Panel::top("topbar")
        .exact_size(TOPBAR_H)
        .show_separator_line(false)
        .frame(bar_frame(theme))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for stage in Tab::ALL {
                    if tab_item(ui, theme, stage.label(), *tab == stage).clicked() {
                        *tab = stage;
                    }
                }
            });
        });
}

/// One stage of the pipeline; the open one is underlined in the accent.
fn tab_item(ui: &mut egui::Ui, theme: &Theme, label: &str, active: bool) -> egui::Response {
    let ink = if active { theme.ink } else { theme.muted };
    let text = ui
        .painter()
        .layout_no_wrap(label.to_owned(), egui::FontId::proportional(13.0), ink);
    let size = egui::vec2(text.size().x + 28.0, ui.available_height());
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    if resp.hovered() && !active {
        ui.painter()
            .rect_filled(rect, 0.0, theme.accent.gamma_multiply(0.07));
    }
    let at = rect.center() - text.size() / 2.0;
    ui.painter().galley(at, text, ink);
    if active {
        let underline = egui::Rect::from_min_max(
            egui::pos2(rect.left(), rect.bottom() - 2.0),
            rect.right_bottom(),
        );
        ui.painter().rect_filled(underline, 0.0, theme.accent);
    }
    resp
}

fn status_bar(ui: &mut egui::Ui, theme: &Theme, tab: Tab, session: &Session) {
    egui::Panel::bottom("status")
        .exact_size(STATUS_H)
        .show_separator_line(false)
        .frame(bar_frame(theme))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for (i, cell) in tab.status(session).iter().enumerate() {
                    if i > 0 {
                        ui.label(cell_text(" · ", theme.line));
                    }
                    ui.label(cell_text(cell, theme.muted));
                }
            });
        });
}

fn cell_text(text: &str, color: egui::Color32) -> egui::RichText {
    egui::RichText::new(text)
        .monospace()
        .size(11.0)
        .color(color)
}
