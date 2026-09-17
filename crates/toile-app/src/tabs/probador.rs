mod seams;

use eframe::egui::{
    self, Align2, Color32, FontId, Painter, Pos2, Rect, Sense, Shape, Stroke, Vec2, pos2, vec2,
};
use eframe::egui_wgpu::RenderState;
use toile_engine::session::Session;

use crate::fitting::Fitting;
use crate::pattern;
use crate::tabs::{UNNAMED, Workspace, right_panel};
use crate::theme::Theme;
use crate::viewport::{Avatar, Viewport};
use crate::widgets::{
    PAD, button_ghost_icon, field_row, footer_note, readout, section, section_with,
};

/// Gap between the 2D and 3D halves, in points.
const SPLIT_GAP: f32 = 12.0;
const SUBBAR_H: f32 = 44.0;
const SEAM_H: f32 = 28.0;
const MARK: f32 = 12.0;

const EMPTY: &str = "El producto en la mesa no lleva costuras.";
/// What the table answers with before any product is opened. The drafting tab
/// calls that same table empty, and a panel here naming a product that is not
/// there would have the two tabs disagreeing about what is on the stand.
const BARE: &str = "No hay ningún producto en la mesa.";
const NOTE: &str = "Editar un punto en 2D re-drapea sin resetear la simulación.";

/// The tab's own state: a GPU viewport and the drag in progress.
pub struct State {
    rs: RenderState,
    viewport: Viewport,
    /// The node being dragged on the 2D half, while one is.
    drag: Option<pattern::Drag>,
    /// Which solve the body on the view came from, so a frame that changed
    /// nothing uploads nothing.
    body_at: Option<u64>,
}

impl State {
    pub fn new(rs: RenderState, theme: &Theme, session: &Session) -> Self {
        // No body until the fitting has solved one. The first frame that finds
        // one puts it behind the cloth.
        let viewport = Viewport::new(
            &rs,
            theme,
            session.n_vertices(),
            session.triangles(),
            Avatar::none(),
        );
        Self {
            rs,
            viewport,
            drag: None,
            body_at: None,
        }
    }
}

pub fn show(ui: &mut egui::Ui, w: &mut Workspace<'_>) {
    let theme = w.theme;
    let body = body_note(w.fitting);
    sub_bar(ui, theme, w.session, &body);
    right_panel(ui, theme, |ui| inspector(ui, theme, w.session));
    egui::CentralPanel::no_frame().show(ui, |ui| {
        let full = ui.available_size();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let half = vec2((full.x - SPLIT_GAP) / 2.0, full.y);
            let st = &mut *w.probador;
            pattern::show(ui, half, theme, w.session, &mut st.drag);
            gutter(ui, theme, full.y);
            if st.body_at != Some(w.fitting.solves())
                && let Some(mesh) = w.fitting.mesh()
            {
                st.viewport
                    .set_avatar(&st.rs, Avatar::body(mesh, theme.avatar));
                st.body_at = Some(w.fitting.solves());
            }
            st.viewport.show(ui, half, &st.rs, theme, w.session);
        });
    });
}

// ── bars and panels ───────────────────────────────────────────────────────

/// The bar over the table: the body being fitted, and the sim controls.
///
/// The body is the only one of the three things a fitting names that the
/// document can answer for. It carries no product name, and the app carries no
/// fabric at all, so a box for either would read the same two words over every
/// pattern ever opened. It is a readout and not a picker, because the document
/// resolves against the body the drafting table chose and this bar has no say.
///
/// The three sim controls are drawn dead. The sim thread takes a rest update,
/// a swapped mesh and a shutdown, and nothing else: there is no pause to ask
/// for, no resume, and no starting state to go back to. They keep their room
/// so that the phase which builds them moves nothing on this bar.
fn sub_bar(ui: &mut egui::Ui, theme: &Theme, session: &Session, body: &str) {
    egui::Panel::top("probador-subbar")
        .exact_size(SUBBAR_H)
        .frame(
            egui::Frame::new()
                .fill(theme.panel)
                .inner_margin(egui::Margin::symmetric(16, 0)),
        )
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                if let Some(named) = fitted(session) {
                    readout(ui, theme, "maniquí", named, 150.0);
                }
                readout(ui, theme, "cuerpo", body, 170.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    button_ghost_icon(ui, theme, "Reiniciar", reset_icon);
                    button_ghost_icon(ui, theme, "Pausar", pause_icon);
                    button_ghost_icon(ui, theme, "Simular", play_icon);
                });
            });
        });
}

/// The seam table of whatever is on the table, and the piece that drapes.
///
/// Every row is measured from the draft on the frame it is drawn. The table is
/// the one place in the app that renders a verdict about someone's own pattern,
/// so it says nothing it did not measure: a seam it cannot walk shows no
/// lengths and carries no mark.
fn inspector(ui: &mut egui::Ui, theme: &Theme, session: &Session) {
    let rows = session.draft().map_or_else(Vec::new, seams::measured);
    section_with(ui, theme, "Costuras", &rows.len().to_string());
    if rows.is_empty() {
        let none = if session.draft().is_some() {
            EMPTY
        } else {
            BARE
        };
        footer_note(ui, theme, none);
    }
    for seam in &rows {
        seam_row(ui, theme, seam);
    }
    for seam in &rows {
        if let Some(complaint) = seam.complaint.as_deref() {
            mismatch(ui, theme, complaint);
        }
    }
    if let Some(name) = draped(session) {
        section(ui, theme, "Pieza en la mesa");
        field_row(ui, theme, "nombre", name, "");
    }
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        footer_note(ui, theme, NOTE);
    });
}

/// What the body on the stand cost, and where it came from.
///
/// The bake runs on its own thread and the drape is never held up waiting for
/// it, so a person who moves a measurement sees the body change before the
/// field behind it does. This is the whole of the report, and it belongs on
/// the bar that names the body rather than in a dialog nobody asked for.
fn body_note(fitting: &Fitting) -> String {
    if let Some(why) = fitting.refused.as_deref() {
        return why.to_owned();
    }
    match &fitting.cost {
        Some(cost) if cost.cached => "en caché".to_owned(),
        Some(cost) => format!("horneado en {:.0} ms", cost.ms),
        None => "horneando…".to_owned(),
    }
}

/// The body the document on the table resolves against, by name.
///
/// A product made from scratch resolves against a body nobody has named yet,
/// and an empty box reads as a box that failed to fill.
fn fitted(session: &Session) -> Option<&str> {
    let doc = session.draft()?.doc();
    let name = &doc.mannequins.get(doc.resolve_with)?.name;
    Some(if name.is_empty() { UNNAMED } else { name })
}

/// The piece draping right now, under the name the document gives it.
fn draped(session: &Session) -> Option<&str> {
    let draft = session.draft()?;
    let held = draft.doc().pieces.get(session.piece()?)?;
    Some(&held.name)
}

/// Seam name, the two lengths, and the mark saying whether they close.
fn seam_row(ui: &mut egui::Ui, theme: &Theme, seam: &seams::Row) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), SEAM_H), Sense::hover());
    let p = ui.painter();
    p.text(
        rect.left_center() + vec2(PAD, 0.0),
        Align2::LEFT_CENTER,
        &seam.name,
        FontId::proportional(12.0),
        theme.ink_soft,
    );
    let slot = Rect::from_center_size(
        rect.right_center() - vec2(PAD + MARK / 2.0, 0.0),
        Vec2::splat(MARK),
    );
    let ink = match seam.meets {
        Some(true) => {
            check_icon(p, slot, theme.accent);
            theme.ink
        }
        Some(false) => {
            warn_icon(p, slot, theme.alert);
            theme.alert
        }
        None => theme.muted,
    };
    p.text(
        pos2(slot.left() - 8.0, rect.center().y),
        Align2::RIGHT_CENTER,
        &seam.lengths,
        FontId::monospace(11.0),
        ink,
    );
}

/// Says in words what the warning mark on the seam row only hints at.
fn mismatch(ui: &mut egui::Ui, theme: &Theme, complaint: &str) {
    let margin = egui::Margin {
        left: 12,
        right: 12,
        top: 6,
        bottom: 10,
    };
    egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        let body = egui::RichText::new(complaint).monospace().size(11.0);
        ui.label(body.color(theme.alert));
    });
}

/// The seam between the two halves of the split, ruled on both sides.
fn gutter(ui: &mut egui::Ui, theme: &Theme, height: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(SPLIT_GAP, height), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 0.0, theme.panel);
    let stroke = Stroke::new(1.0, theme.line);
    p.line_segment([rect.left_top(), rect.left_bottom()], stroke);
    p.line_segment([rect.right_top(), rect.right_bottom()], stroke);
}

// ── glyphs ────────────────────────────────────────────────────────────────

fn play_icon(p: &Painter, r: Rect, color: Color32) {
    let b = r.shrink(1.0);
    let tip = pos2(b.right(), b.center().y);
    let points = vec![b.left_top(), tip, b.left_bottom()];
    p.add(Shape::convex_polygon(points, color, Stroke::NONE));
}

fn pause_icon(p: &Painter, r: Rect, color: Color32) {
    let b = r.shrink2(vec2(2.5, 1.0));
    let stroke = Stroke::new(1.6, color);
    p.line_segment([b.left_top(), b.left_bottom()], stroke);
    p.line_segment([b.right_top(), b.right_bottom()], stroke);
}

/// An almost closed circle, with the corner mark of an arrowhead at its start.
fn reset_icon(p: &Painter, r: Rect, color: Color32) {
    let at = |x: f32, y: f32| r.left_top() + vec2(x, y) * r.width() / 16.0;
    let stroke = Stroke::new(1.3, color);
    let arc: Vec<Pos2> = (0..=16)
        .map(|i| {
            let a = (180.0 - 315.0 * i as f32 / 16.0).to_radians();
            at(8.0 + 5.0 * a.cos(), 8.0 + 5.0 * a.sin())
        })
        .collect();
    p.add(Shape::line(arc, stroke));
    p.add(Shape::line(
        vec![at(3.0, 3.0), at(3.0, 6.0), at(6.0, 6.0)],
        stroke,
    ));
}

fn check_icon(p: &Painter, r: Rect, color: Color32) {
    let at = |x: f32, y: f32| r.left_top() + vec2(x, y) * r.width() / 12.0;
    let tick = vec![at(2.5, 6.5), at(5.0, 9.5), at(9.5, 3.0)];
    p.add(Shape::line(tick, Stroke::new(1.5, color)));
}

fn warn_icon(p: &Painter, r: Rect, color: Color32) {
    let at = |x: f32, y: f32| r.left_top() + vec2(x, y) * r.width() / 12.0;
    let stroke = Stroke::new(1.2, color);
    let body = vec![at(6.0, 1.5), at(11.0, 10.5), at(1.0, 10.5)];
    p.add(Shape::closed_line(body, stroke));
    p.line_segment([at(6.0, 5.0), at(6.0, 8.5)], stroke);
}

#[cfg(test)]
mod tests;
