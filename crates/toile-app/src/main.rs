#![allow(missing_docs, reason = "a binary publishes no API surface")]

mod camera;
mod pattern;
mod render;
mod viewport;

use eframe::egui;
use toile_engine::session::Session;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1320.0, 780.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native("Toile", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}

/// Gap between the two panels, in points.
const SPLIT_GAP: f32 = 12.0;

struct App {
    session: Session,
    rs: eframe::egui_wgpu::RenderState,
    viewport: viewport::Viewport,
    /// Contour point currently being dragged, if any.
    drag: Option<usize>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let session = Session::demo_bodice();
        let rs = cc
            .wgpu_render_state
            .clone()
            .expect("eframe was configured with the wgpu renderer");
        let viewport = viewport::Viewport::new(
            &rs,
            session.n_vertices(),
            session.triangles(),
            session.avatar_radius(),
        );
        Self {
            session,
            rs,
            viewport,
            drag: None,
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::bottom("status").show(ui, |ui| {
            let snap = self.session.snapshot();
            ui.horizontal(|ui| {
                ui.label(format!("substeps {}", snap.substeps));
                ui.separator();
                ui.label(if snap.converged {
                    "sim dormida (0% CPU)"
                } else {
                    "sim corriendo"
                });
                ui.separator();
                ui.label(format!("derive {:.1} ms", self.session.last_derive_ms));
            });
        });
        egui::CentralPanel::default().show(ui, |ui| {
            let full = ui.available_size();
            ui.horizontal(|ui| {
                let half = egui::vec2((full.x - SPLIT_GAP) * 0.5, full.y);
                pattern::show(ui, half, &mut self.session, &mut self.drag);
                let snap = self.session.snapshot();
                self.viewport.show(ui, half, &self.rs, &snap);
            });
        });
        // The sim advances on its own clock; without this the viewport would
        // only redraw on input events.
        ui.ctx().request_repaint();
    }
}
