mod icons;
mod note;
mod seams;

use eframe::egui::{self, Align2, FontId, Rect, Sense, Stroke, Vec2, pos2, vec2};
use eframe::egui_wgpu::RenderState;
use icons::{check_icon, pause_icon, play_icon, reset_icon, warn_icon};
use toile_engine::draft::BodyMesh;
use toile_engine::session::Session;

use crate::pattern;
use crate::tabs::{UNNAMED, Workspace, right_panel};
use crate::theme::Theme;
use crate::viewport::{Avatar, Viewport};
use crate::widgets::{
    PAD, alert_note, button_ghost_icon, field_row, footer_note, readout, section_with,
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
    let body = note::cost(w.fitting);
    let crossed = w.fitting.crossings.as_ref().and_then(note::crossed);
    sub_bar(ui, theme, w.session, &body, crossed.as_deref());
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
                    .set_avatar(&st.rs, standing(w.session, mesh, theme));
                st.body_at = Some(w.fitting.solves());
            }
            st.viewport.show(ui, half, &st.rs, theme, w.session);
        });
    });
}

/// The body as the viewport draws it, on the ground it stands on.
///
/// The plane is the solver's own, read off the body the drape is falling on,
/// so the surface a person watches the garment land on is the one that
/// stopped it. A body that stands on nothing — the demo sphere, which is the
/// physics reference and floats — is drawn without any.
fn standing(session: &Session, mesh: &BodyMesh, theme: &Theme) -> Avatar {
    let avatar = Avatar::body(mesh, theme.avatar);
    let Some(y) = session.collider().ground() else {
        return avatar;
    };
    let (lo, hi) = session.collider().extent();
    let centre = [f32::midpoint(lo[0], hi[0]), f32::midpoint(lo[2], hi[2])];
    avatar.standing_on(y, centre, theme.ground)
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
fn sub_bar(ui: &mut egui::Ui, theme: &Theme, session: &Session, body: &str, crossed: Option<&str>) {
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
                // In the ink of any other readout: it is something known about
                // the body, and the body was baked and is draped on all the
                // same.
                if let Some(crossed) = crossed {
                    readout(ui, theme, "cruces", crossed, 190.0);
                }
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
            alert_note(ui, theme, complaint);
        }
    }
    // A seam the engine could not pair is not draping at all, which is a
    // louder thing than two sides that do not close, and is said the same way.
    if let Some(draft) = session.draft() {
        for why in seams::refused(draft, session.seam_faults()) {
            alert_note(ui, theme, &why);
        }
    }
    let draped = draped(session);
    if !draped.is_empty() {
        section_with(ui, theme, "Piezas en la mesa", &draped.len().to_string());
    }
    for name in draped {
        field_row(ui, theme, "pieza", name, "");
    }
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        footer_note(ui, theme, NOTE);
    });
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

/// The pieces draping right now, under the names the document gives them.
///
/// Every piece of the product is on the stand, so this is a list and not a
/// name: a panel that showed one of them would be naming whichever happens to
/// come first.
fn draped(session: &Session) -> Vec<&str> {
    let Some(draft) = session.draft() else {
        return Vec::new();
    };
    let named = |key| Some(draft.doc().pieces.get(key)?.name.as_str());
    session.pieces().into_iter().filter_map(named).collect()
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

/// The seam between the two halves of the split, ruled on both sides.
fn gutter(ui: &mut egui::Ui, theme: &Theme, height: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(SPLIT_GAP, height), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 0.0, theme.panel);
    let stroke = Stroke::new(1.0, theme.line);
    p.line_segment([rect.left_top(), rect.left_bottom()], stroke);
    p.line_segment([rect.right_top(), rect.right_bottom()], stroke);
}

#[cfg(test)]
mod tests;
