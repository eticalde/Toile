//! Cliente de escritorio v0 — la vista split (issue #21).
//!
//! Izquierda: el patrón 2D con puntos de control arrastrables (vía A en
//! vivo). Derecha: la tela drapeada, sombreada por CPU vía `egui::Mesh`
//! con orden de pintor — placeholder deliberado del renderer wgpu del ADR
//! §2.6, suficiente para 24k triángulos. Solo importa `toile-engine`.

use eframe::egui;
use toile_engine::session::Session;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1320.0, 780.0]),
        ..Default::default()
    };
    eframe::run_native("Toile", options, Box::new(|_cc| Ok(Box::new(App::new()))))
}

struct App {
    session: Session,
    drag: Option<usize>,
    yaw: f32,
    pitch: f32,
    dist: f32,
}

impl App {
    fn new() -> Self {
        Self {
            session: Session::demo_bodice(),
            drag: None,
            yaw: 0.7,
            pitch: 0.35,
            dist: 1.15,
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
        let (resp, painter) = ui.allocate_painter(size, egui::Sense::drag());
        let rect = resp.rect;
        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(17, 22, 20));

        if resp.dragged() {
            self.yaw += resp.drag_delta().x * 0.01;
            self.pitch = (self.pitch + resp.drag_delta().y * 0.01).clamp(-1.4, 1.4);
        }
        if resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.dist = (self.dist * (1.0 - scroll * 0.002)).clamp(0.35, 4.0);
            }
        }

        // Cámara orbital mirando al centro del avatar.
        let target = [0.0f32, 0.02, 0.0];
        let (sy, cy2) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let eye = [
            target[0] + self.dist * cp * sy,
            target[1] + self.dist * sp,
            target[2] + self.dist * cp * cy2,
        ];
        let fwd = norm3(sub3(target, eye));
        let right = norm3(cross3(fwd, [0.0, 1.0, 0.0]));
        let up = cross3(right, fwd);
        let focal = 1.1 * f32::min(rect.width(), rect.height());
        let center = rect.center();
        let project = |p: [f32; 3]| -> ([f32; 2], f32) {
            let d = sub3(p, eye);
            let z = dot3(d, fwd);
            let x = dot3(d, right);
            let y = dot3(d, up);
            ([center.x + focal * x / z, center.y - focal * y / z], z)
        };

        // Avatar: silueta de la esfera, detrás de la tela.
        let r = self.session.avatar_radius();
        let (c, zc) = project([0.0, 0.0, 0.0]);
        if zc > 0.05 {
            painter.circle_filled(
                egui::pos2(c[0], c[1]),
                focal * r / zc,
                egui::Color32::from_rgb(52, 58, 56),
            );
        }

        let snap = self.session.snapshot();
        if snap.positions.is_empty() {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "drapeando…",
                egui::FontId::proportional(16.0),
                egui::Color32::GRAY,
            );
            return;
        }

        // Malla sombreada plana con orden de pintor (lejos → cerca).
        let pos = &snap.positions;
        let tris = self.session.triangles();
        let light = norm3([0.35, 0.8, 0.45]);
        let mut faces: Vec<(f32, [egui::Pos2; 3], egui::Color32)> =
            Vec::with_capacity(tris.len() / 3);
        for t in tris.chunks(3) {
            let v = |k: usize| {
                let i = t[k] as usize * 3;
                [pos[i], pos[i + 1], pos[i + 2]]
            };
            let (a, b, c3) = (v(0), v(1), v(2));
            let n = norm3(cross3(sub3(b, a), sub3(c3, a)));
            let shade = 0.25 + 0.75 * dot3(n, light).abs();
            let (pa, za) = project(a);
            let (pb, zb) = project(b);
            let (pc, zc3) = project(c3);
            if za <= 0.05 || zb <= 0.05 || zc3 <= 0.05 {
                continue;
            }
            let color = egui::Color32::from_rgb(
                (215.0 * shade) as u8,
                (128.0 * shade) as u8,
                (92.0 * shade) as u8,
            );
            faces.push((
                (za + zb + zc3) / 3.0,
                [
                    egui::pos2(pa[0], pa[1]),
                    egui::pos2(pb[0], pb[1]),
                    egui::pos2(pc[0], pc[1]),
                ],
                color,
            ));
        }
        faces.sort_by(|x, y| y.0.total_cmp(&x.0));
        let mut mesh = egui::Mesh::default();
        for (_, p, color) in &faces {
            let base = mesh.vertices.len() as u32;
            for q in p {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: *q,
                    uv: egui::epaint::WHITE_UV,
                    color: *color,
                });
            }
            mesh.indices.extend_from_slice(&[base, base + 1, base + 2]);
        }
        painter.add(egui::Shape::mesh(mesh));
        painter.text(
            rect.left_top() + egui::vec2(10.0, 8.0),
            egui::Align2::LEFT_TOP,
            "3D — arrastra para orbitar · rueda para zoom",
            egui::FontId::monospace(11.0),
            egui::Color32::from_rgb(140, 145, 150),
        );
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
