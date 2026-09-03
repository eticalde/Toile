use eframe::egui;
use eframe::egui_wgpu::RenderState;
use toile_engine::sync::Snapshot;

use crate::camera::{Camera, norm3};
use crate::render;

const CLOTH_COLOR: [f32; 3] = [0.86, 0.52, 0.37];
const LIGHT_DIR: [f32; 3] = [0.35, 0.8, 0.45];
const LABEL: egui::Color32 = egui::Color32::from_rgb(140, 145, 150);

/// Floats per vertex in the renderer's buffer: position, normal, colour.
const VERTEX_FLOATS: usize = 9;

/// The 3D half of the split view.
pub struct Viewport {
    renderer: render::Renderer,
    camera: Camera,
    vertices: Vec<f32>,
}

impl Viewport {
    pub fn new(rs: &RenderState, n_verts: usize, tris: &[u32], avatar_radius: f32) -> Self {
        Self {
            renderer: render::Renderer::new(rs, n_verts, tris, avatar_radius),
            camera: Camera::default(),
            vertices: vec![0.0; n_verts * VERTEX_FLOATS],
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, size: egui::Vec2, rs: &RenderState, snap: &Snapshot) {
        if !snap.positions.is_empty() {
            self.upload(snap);
            let ppp = ui.ctx().pixels_per_point();
            let uniforms = self.uniforms(size.x / size.y.max(1.0));
            self.renderer.paint(
                rs,
                (size.x * ppp) as u32,
                (size.y * ppp) as u32,
                &self.vertices,
                &uniforms,
            );
        }

        let Some(tex) = self.renderer.texture_id else {
            ui.allocate_space(size);
            return;
        };
        let resp = ui.add(egui::Image::from_texture((tex, size)).sense(egui::Sense::drag()));
        if resp.dragged() {
            self.camera.orbit(resp.drag_delta().x, resp.drag_delta().y);
        }
        if resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.camera.zoom(scroll);
            }
        }
        ui.painter().text(
            resp.rect.left_top() + egui::vec2(10.0, 8.0),
            egui::Align2::LEFT_TOP,
            "3D — arrastra para orbitar · rueda para zoom",
            egui::FontId::monospace(11.0),
            LABEL,
        );
    }

    /// Interleaves the snapshot into the renderer's vertex layout.
    fn upload(&mut self, snap: &Snapshot) {
        let n = snap.positions.len() / 3;
        self.vertices.resize(n * VERTEX_FLOATS, 0.0);
        for i in 0..n {
            let dst = &mut self.vertices[i * VERTEX_FLOATS..(i + 1) * VERTEX_FLOATS];
            dst[..3].copy_from_slice(&snap.positions[i * 3..i * 3 + 3]);
            dst[3..6].copy_from_slice(&snap.normals[i * 3..i * 3 + 3]);
            dst[6..9].copy_from_slice(&CLOTH_COLOR);
        }
    }

    fn uniforms(&self, aspect: f32) -> [f32; 20] {
        let mut u = [0.0f32; 20];
        u[..16].copy_from_slice(&self.camera.mvp(aspect));
        u[16..19].copy_from_slice(&norm3(LIGHT_DIR));
        u
    }
}
