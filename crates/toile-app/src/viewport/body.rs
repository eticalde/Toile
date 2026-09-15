use eframe::egui;
use eframe::egui_wgpu::RenderState;
use toile_engine::body::Tape;
use toile_engine::draft::BodyMesh;

use super::camera::{Camera, norm3};
use super::render::{SolidRenderer, ribbon};
use super::{LIGHT_DIR, VERTEX_FLOATS};
use crate::theme::Theme;

/// How far a drawn tape stands off the skin, in metres.
///
/// The tape's points lie on the mesh's own edges, so with no gap the tape would
/// fight the body for every pixel of the depth buffer. Three millimetres
/// also covers the skin bulging toward the camera across the tape's width,
/// down to a curve as tight as the wrist's.
const TAPE_GAP_M: f32 = 0.003;

/// Half the drawn tape's width, in points: four across, wide enough to read
/// as a tape rather than a hairline at the default distance, where a
/// millimetre of body is under a quarter of a point.
const TAPE_HALF_WIDTH: f32 = 2.0;

/// The 3D body view: an orbit camera over a mesh that is regenerated whenever a
/// measurement changes and re-uploaded through [`BodyView::set_mesh`], with the
/// tape of the measurement in hand laid over it through [`BodyView::set_tape`].
///
/// Unlike [`super::Viewport`], nothing here reads a [`Session`]: the body is a
/// static mesh the tab hands over, painted afresh each frame so orbit and zoom
/// follow the camera without a solver in the loop.
///
/// [`Session`]: toile_engine::session::Session
pub struct BodyView {
    renderer: SolidRenderer,
    camera: Camera,
    /// The interleaved 9-float vertices, kept so a re-upload reuses the
    /// allocation.
    verts: Vec<f32>,
    color: [f32; 3],
    tape: [f32; 3],
}

impl BodyView {
    pub fn new(rs: &RenderState, theme: &Theme) -> Self {
        Self {
            renderer: SolidRenderer::new(rs, theme.clear_color()),
            camera: Camera::for_body(),
            verts: Vec::new(),
            color: theme.avatar,
            tape: theme.accent_rgb(),
        }
    }

    /// Interleaves the mesh into the renderer's position/normal/colour layout
    /// and uploads it, growing the buffers only when the mesh grows.
    pub fn set_mesh(&mut self, rs: &RenderState, mesh: &BodyMesh) {
        let n = mesh.vertex_count();
        self.verts.resize(n * VERTEX_FLOATS, 0.0);
        for i in 0..n {
            let dst = &mut self.verts[i * VERTEX_FLOATS..(i + 1) * VERTEX_FLOATS];
            dst[..3].copy_from_slice(&mesh.positions[i * 3..i * 3 + 3]);
            dst[3..6].copy_from_slice(&mesh.normals[i * 3..i * 3 + 3]);
            dst[6..9].copy_from_slice(&self.color);
        }
        self.renderer.upload(rs, &self.verts, &mesh.indices);
    }

    /// Lays `tape` over the body, or takes the tape away. Uploads the tape
    /// alone: the body's buffers are not touched.
    pub fn set_tape(&mut self, rs: &RenderState, tape: Option<&Tape>) {
        let strip = tape.map_or_else(Vec::new, |t| ribbon(&t.lifted(TAPE_GAP_M), t.closed));
        self.renderer.upload_tape(rs, &strip);
    }

    /// Paints the body at the current camera and shows it, taking drag as
    /// orbit and the wheel as zoom.
    pub fn show(&mut self, ui: &mut egui::Ui, size: egui::Vec2, rs: &RenderState, theme: &Theme) {
        let ppp = ui.ctx().pixels_per_point();
        let (w, h) = ((size.x * ppp) as u32, (size.y * ppp) as u32);
        let mvp = self.camera.mvp(size.x / size.y.max(1.0));
        let mut body = [0.0f32; 20];
        body[..16].copy_from_slice(&mvp);
        body[16..19].copy_from_slice(&norm3(LIGHT_DIR));
        let mut tape = [0.0f32; 24];
        tape[..16].copy_from_slice(&mvp);
        // The target is never smaller than 8 pixels a side; the widening
        // divides by what is really there.
        tape[16..19].copy_from_slice(&[w.max(8) as f32, h.max(8) as f32, TAPE_HALF_WIDTH * ppp]);
        tape[20..23].copy_from_slice(&self.tape);
        tape[23] = 1.0;
        self.renderer.paint(rs, w, h, &body, &tape);

        super::steer(
            ui,
            theme,
            size,
            self.renderer.texture_id(),
            &mut self.camera,
        );
    }
}
