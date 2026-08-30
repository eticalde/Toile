//! Cliente de escritorio — la vista split (issue #21).
//!
//! Izquierda: el patrón 2D con puntos de control arrastrables (vía A en
//! vivo). Derecha: la tela drapeada por el renderer wgpu propio (módulo
//! `render`, ADR §2.6) — offscreen con depth buffer, sombreado suave con
//! las normales que publica el hilo de sim. Solo importa `toile-engine`.

mod render;

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

const CLOTH_COLOR: [f32; 3] = [0.86, 0.52, 0.37];

struct App {
    session: Session,
    rs: eframe::egui_wgpu::RenderState,
    renderer: render::Renderer,
    drag: Option<usize>,
    yaw: f32,
    pitch: f32,
    dist: f32,
    cloth_vertices: Vec<f32>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let session = Session::demo_bodice();
        let rs = cc
            .wgpu_render_state
            .clone()
            .expect("eframe sin backend wgpu");
        let n_verts = session.n_vertices();
        let renderer =
            render::Renderer::new(&rs, n_verts, session.triangles(), session.avatar_radius());
        Self {
            session,
            rs,
            renderer,
            drag: None,
            yaw: 0.7,
            pitch: 0.35,
            dist: 1.15,
            cloth_vertices: vec![0.0; n_verts * 9],
        }
    }

    fn pattern_panel(&mut self, ui: &mut egui::Ui, size: egui::Vec2) {
        let (resp, painter) = ui.allocate_painter(size, egui::Sense::click_and_drag());
        let rect = resp.rect;
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(24, 26, 30));

        // Encaje del patrón (metros, y hacia arriba) en el rect, con margen.
        let contour: Vec<[f64; 2]> = self.session.contour().to_vec();
        let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
        for p in &contour {
            for k in 0..2 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        let span = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1.0e-6);
        let margin = 40.0;
        let scale = (f64::from(rect.width().min(rect.height())) - 2.0 * margin) / span;
        let (cx, cy) = ((lo[0] + hi[0]) * 0.5, (lo[1] + hi[1]) * 0.5);
        let center = rect.center();
        let to_screen = |p: [f64; 2]| {
            egui::pos2(
                center.x + ((p[0] - cx) * scale) as f32,
                center.y - ((p[1] - cy) * scale) as f32,
            )
        };
        let from_screen = |q: egui::Pos2| {
            [
                cx + f64::from(q.x - center.x) / scale,
                cy - f64::from(q.y - center.y) / scale,
            ]
        };

        let pts: Vec<egui::Pos2> = contour.iter().map(|&p| to_screen(p)).collect();
        painter.add(egui::Shape::closed_line(
            pts.clone(),
            egui::Stroke::new(1.6, egui::Color32::from_rgb(210, 210, 200)),
        ));

        // Arrastre: al iniciar, el punto de control más cercano (radio 14 px).
        if resp.drag_started()
            && let Some(pos) = resp.interact_pointer_pos()
        {
            let mut best = (14.0f32, None);
            for (i, q) in pts.iter().enumerate() {
                let d = q.distance(pos);
                if d < best.0 {
                    best = (d, Some(i));
                }
            }
            self.drag = best.1;
        }
        if resp.drag_stopped() {
            self.drag = None;
        }
        if let Some(i) = self.drag
            && let Some(pos) = resp.interact_pointer_pos()
        {
            self.session.move_point(i, from_screen(pos));
        }

        for (i, q) in pts.iter().enumerate() {
            let (r, color) = if self.drag == Some(i) {
                (5.0, egui::Color32::from_rgb(235, 90, 80))
            } else {
                (2.4, egui::Color32::from_rgb(95, 140, 235))
            };
            painter.circle_filled(*q, r, color);
        }
        painter.text(
            rect.left_top() + egui::vec2(10.0, 8.0),
            egui::Align2::LEFT_TOP,
            "PATRÓN 2D — arrastra un punto",
            egui::FontId::monospace(11.0),
            egui::Color32::from_rgb(140, 145, 150),
        );
    }

    fn viewport_panel(&mut self, ui: &mut egui::Ui, size: egui::Vec2) {
        let snap = self.session.snapshot();
        let ppp = ui.ctx().pixels_per_point();
        let (w, h) = ((size.x * ppp) as u32, (size.y * ppp) as u32);

        if !snap.positions.is_empty() {
            // Intercalar pos + normal + color para el frame.
            let n = snap.positions.len() / 3;
            self.cloth_vertices.resize(n * 9, 0.0);
            for i in 0..n {
                let dst = &mut self.cloth_vertices[i * 9..i * 9 + 9];
                dst[..3].copy_from_slice(&snap.positions[i * 3..i * 3 + 3]);
                dst[3..6].copy_from_slice(&snap.normals[i * 3..i * 3 + 3]);
                dst[6..9].copy_from_slice(&CLOTH_COLOR);
            }

            // Cámara orbital → MVP (wgpu clip z ∈ [0,1]).
            let target = [0.0f32, 0.02, 0.0];
            let (sy, cy) = self.yaw.sin_cos();
            let (sp, cp) = self.pitch.sin_cos();
            let eye = [
                target[0] + self.dist * cp * sy,
                target[1] + self.dist * sp,
                target[2] + self.dist * cp * cy,
            ];
            let view = look_at(eye, target, [0.0, 1.0, 0.0]);
            let proj = perspective(55f32.to_radians(), size.x / size.y.max(1.0), 0.02, 20.0);
            let mvp = mul4(proj, view);
            let light = norm3([0.35, 0.8, 0.45]);
            let mut uniforms = [0.0f32; 20];
            uniforms[..16].copy_from_slice(&mvp);
            uniforms[16..19].copy_from_slice(&light);

            self.renderer
                .paint(&self.rs, w, h, &self.cloth_vertices, &uniforms);
        }

        if let Some(tex) = self.renderer.texture_id {
            let resp = ui.add(egui::Image::from_texture((tex, size)).sense(egui::Sense::drag()));
            if resp.dragged() {
                self.yaw += resp.drag_delta().x * 0.01;
                self.pitch = (self.pitch + resp.drag_delta().y * 0.01).clamp(-1.4, 1.4);
            }
            if resp.hovered() {
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll != 0.0 {
                    self.dist = (self.dist * (1.0 - scroll * 0.002)).clamp(0.3, 4.0);
                }
            }
            ui.painter().text(
                resp.rect.left_top() + egui::vec2(10.0, 8.0),
                egui::Align2::LEFT_TOP,
                "3D — arrastra para orbitar · rueda para zoom",
                egui::FontId::monospace(11.0),
                egui::Color32::from_rgb(140, 145, 150),
            );
        } else {
            ui.allocate_space(size);
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
                let half = egui::vec2((full.x - 12.0) * 0.5, full.y);
                self.pattern_panel(ui, half);
                self.viewport_panel(ui, half);
            });
        });
        // La sim corre en su hilo: repintar continuamente para verla.
        ui.ctx().request_repaint();
    }
}

// ── álgebra mínima (column-major, sin dependencias) ──────────────────────

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm3(a: [f32; 3]) -> [f32; 3] {
    let l = dot3(a, a).sqrt().max(1.0e-9);
    [a[0] / l, a[1] / l, a[2] / l]
}

#[rustfmt::skip]
fn look_at(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [f32; 16] {
    let f = norm3(sub3(target, eye));
    let s = norm3(cross3(f, up));
    let u = cross3(s, f);
    [
        s[0], u[0], -f[0], 0.0,
        s[1], u[1], -f[1], 0.0,
        s[2], u[2], -f[2], 0.0,
        -dot3(s, eye), -dot3(u, eye), dot3(f, eye), 1.0,
    ]
}

/// Proyección perspectiva RH con clip z ∈ [0, 1] (convención wgpu).
fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> [f32; 16] {
    let f = 1.0 / (fov_y * 0.5).tan();
    let mut m = [0.0f32; 16];
    m[0] = f / aspect;
    m[5] = f;
    m[10] = far / (near - far);
    m[11] = -1.0;
    m[14] = near * far / (near - far);
    m
}

fn mul4(a: [f32; 16], b: [f32; 16]) -> [f32; 16] {
    let mut m = [0.0f32; 16];
    for col in 0..4 {
        for row in 0..4 {
            let mut acc = 0.0;
            for k in 0..4 {
                acc += a[k * 4 + row] * b[col * 4 + k];
            }
            m[col * 4 + row] = acc;
        }
    }
    m
}
