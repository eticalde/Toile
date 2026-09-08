use eframe::egui;
use eframe::egui_wgpu::RenderState;
use toile_engine::draft::BodyMesh;

use super::camera::{Camera, norm3};
use super::render::SolidRenderer;
use super::{LIGHT_DIR, VERTEX_FLOATS};
use crate::theme::Theme;
use crate::widgets::canvas_label;

/// The 3D body view: an orbit camera over a mesh that is regenerated whenever a
/// measurement changes and re-uploaded through [`BodyView::set_mesh`], and
/// recoloured — without relofting — to light the stations a measurement is
/// read at through [`BodyView::set_highlight`].
///
/// Unlike [`super::Viewport`], nothing here reads a [`Session`]: the body is a
/// static mesh the tab hands over, painted afresh each frame so orbit and zoom
/// follow the camera without a solver in the loop.
///
/// [`Session`]: toile_engine::session::Session
pub struct BodyView {
    renderer: SolidRenderer,
    camera: Camera,
    /// The interleaved 9-float vertices, kept so a recolour rewrites only the
    /// colour floats and a re-upload reuses the buffer.
    verts: Vec<f32>,
    /// The mesh's indices, kept for the re-upload a recolour needs.
    indices: Vec<u32>,
    /// The station tag of every vertex, which a highlight mask selects by.
    stations: Vec<u8>,
    color: [f32; 3],
    lit: [f32; 3],
}

impl BodyView {
    pub fn new(rs: &RenderState, theme: &Theme) -> Self {
        let accent = theme.accent;
        Self {
            renderer: SolidRenderer::new(rs, theme.clear_color()),
            camera: Camera::for_body(),
            verts: Vec::new(),
            indices: Vec::new(),
            stations: Vec::new(),
            color: theme.avatar,
            lit: [accent.r(), accent.g(), accent.b()].map(|c| f32::from(c) / 255.0),
        }
    }

    /// Interleaves the mesh into the renderer's position/normal/colour layout,
    /// lit by `mask`, and uploads it, growing the buffers only when the mesh
    /// grows.
    pub fn set_mesh(&mut self, rs: &RenderState, mesh: &BodyMesh, mask: u32) {
        let n = mesh.vertex_count();
        self.verts.resize(n * VERTEX_FLOATS, 0.0);
        for i in 0..n {
            let dst = &mut self.verts[i * VERTEX_FLOATS..(i + 1) * VERTEX_FLOATS];
            dst[..3].copy_from_slice(&mesh.positions[i * 3..i * 3 + 3]);
            dst[3..6].copy_from_slice(&mesh.normals[i * 3..i * 3 + 3]);
        }
        self.indices.clone_from(&mesh.indices);
        self.stations.clone_from(&mesh.stations);
        self.set_highlight(rs, mask);
    }

    /// Lights every vertex whose station is in `mask` (one bit per tag) and
    /// re-uploads. A recolour only: the loft is not run again.
    pub fn set_highlight(&mut self, rs: &RenderState, mask: u32) {
        for (i, &tag) in self.stations.iter().enumerate() {
            let hue = if mask & (1 << u32::from(tag)) != 0 {
                self.lit
            } else {
                self.color
            };
            let at = i * VERTEX_FLOATS + 6;
            self.verts[at..at + 3].copy_from_slice(&hue);
        }
        self.renderer.upload(rs, &self.verts, &self.indices);
    }

    /// Paints the body at the current camera and shows it, taking drag as
    /// orbit and the wheel as zoom.
    pub fn show(&mut self, ui: &mut egui::Ui, size: egui::Vec2, rs: &RenderState, theme: &Theme) {
        let ppp = ui.ctx().pixels_per_point();
        let uniforms = self.uniforms(size.x / size.y.max(1.0));
        self.renderer
            .paint(rs, (size.x * ppp) as u32, (size.y * ppp) as u32, &uniforms);

        let Some(tex) = self.renderer.texture_id() else {
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
        canvas_label(
            ui.painter(),
            theme,
            resp.rect,
            "3D — arrastra para orbitar · rueda para zoom",
        );
    }

    fn uniforms(&self, aspect: f32) -> [f32; 20] {
        let mut u = [0.0f32; 20];
        u[..16].copy_from_slice(&self.camera.mvp(aspect));
        u[16..19].copy_from_slice(&norm3(LIGHT_DIR));
        u
    }
}
