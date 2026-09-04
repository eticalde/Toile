use eframe::egui::{
    self, Align2, FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, pos2, vec2,
};
use toile_engine::draft::{Draft, PieceKey, PointKey};

use super::state::State;
use super::view::View;
use super::{input, paper, ruler};
use crate::glyph;
use crate::theme::Theme;
use crate::widgets::{PAD, button_icon, button_secondary, canvas_label, fill, grid};

/// The closest the mat draws its lines; under that they read as noise.
const GRID_MIN: f32 = 9.0;

/// How much one notch of the wheel is worth in scale.
const ZOOM_RATE: f32 = 0.004;

/// The extremes a single wheel event may move the scale by.
const ZOOM_LIMIT: [f64; 2] = [0.2, 5.0];

const TAG: &str = "2 4 10 4 14 8 10 12 2 12 2 4; o 5 8 1.3";
const HINT: &str = "Mesa vacía — carga «Ejemplo · pantalón base» desde Producto";

/// The cutting mat: the piece at whatever scale the view holds, its rulers,
/// and the chips that move it.
pub fn show(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: Option<&Draft>,
    piece: Option<PieceKey>,
    state: &mut State,
) {
    egui::CentralPanel::no_frame().show(ui, |ui| {
        let size = ui.available_size();
        let (resp, painter) = ui.allocate_painter(size, Sense::click_and_drag());
        let rect = resp.rect;
        let nodes: &[(PointKey, [f64; 2])] = match (draft, piece) {
            (Some(draft), Some(piece)) => draft.points_cm(piece),
            _ => &[],
        };
        let inner = Rect::from_min_max(
            rect.left_top() + vec2(ruler::BAND, ruler::BAND),
            rect.right_bottom(),
        );
        if state.frame
            && let Some(bbox) = input::bounds(nodes)
        {
            state.view.fit(bbox, inner);
            state.frame = false;
        }
        interact(ui, &resp, nodes, state);
        fill(&painter, theme, rect);
        mat_grid(&painter, theme, rect, state.view);
        let over = resp
            .hover_pos()
            .and_then(|at| input::pick(nodes, state.view, at));
        if let (Some(draft), Some(piece)) = (draft, piece) {
            paper_and_outline(&painter, theme, draft, piece, state.view);
            marks(&painter, theme, draft, piece, state, over);
        } else {
            let font = FontId::proportional(13.0);
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                HINT,
                font,
                theme.muted,
            );
        }
        ruler::show(&painter, theme, rect, state.view);
        let zoom = state.view.zoom_percent(ui.ctx().pixels_per_point());
        caption(&painter, theme, rect, &name_of(draft, piece), zoom);
        chips(ui, theme, rect, state);
    });
}

/// Wheel, drag and keys, against the view and the selection.
fn interact(ui: &egui::Ui, resp: &Response, nodes: &[(PointKey, [f64; 2])], state: &mut State) {
    if resp.dragged() {
        state.view.pan(resp.drag_delta());
    }
    if let Some(at) = resp.hover_pos() {
        let (wheel, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
        let factor = f64::from((1.0 + wheel * ZOOM_RATE) * pinch);
        if (factor - 1.0).abs() > 1.0e-6 {
            state
                .view
                .zoom_at(at, factor.clamp(ZOOM_LIMIT[0], ZOOM_LIMIT[1]));
        }
    }
    if resp.clicked()
        && let Some(at) = resp.interact_pointer_pos()
    {
        state.selection = input::pick(nodes, state.view, at);
    }
    let ppp = ui.ctx().pixels_per_point();
    ui.input(|i| {
        if i.key_pressed(egui::Key::F) {
            state.frame = true;
        }
        if i.key_pressed(egui::Key::Num1) {
            state.view.one_to_one(ppp);
        }
        if i.key_pressed(egui::Key::Escape) {
            state.selection = None;
        }
    });
}

/// The ruled lines, travelling with the view so a centimetre stays a
/// centimetre wherever the drawing has been dragged to.
fn mat_grid(p: &Painter, theme: &Theme, rect: Rect, view: View) {
    let step = (ruler::step_cm(view.scale()) * view.scale() / 2.0) as f32;
    if step < GRID_MIN {
        return;
    }
    grid(
        p,
        theme,
        rect,
        step,
        view.to_screen([0.0, 0.0]) - rect.left_top(),
    );
}

/// The piece itself: paper under an outline that turns to alert ink when the
/// contour has stopped being one.
fn paper_and_outline(p: &Painter, theme: &Theme, draft: &Draft, piece: PieceKey, view: View) {
    let cm: Vec<[f64; 2]> = draft.points_cm(piece).iter().map(|&(_, at)| at).collect();
    if cm.len() < 3 {
        return;
    }
    let pts: Vec<Pos2> = cm.iter().map(|&at| view.to_screen(at)).collect();
    let mut mesh = egui::Mesh::default();
    for at in &pts {
        mesh.colored_vertex(*at, theme.paper);
    }
    for [a, b, c] in paper::triangles(&cm) {
        mesh.add_triangle(a as u32, b as u32, c as u32);
    }
    p.add(Shape::mesh(mesh));
    let ink = if draft.defects(piece).is_empty() {
        theme.outline
    } else {
        theme.alert
    };
    p.add(Shape::closed_line(pts, Stroke::new(1.5, ink)));
}

/// The nodes, and the names of the ones asking to be read.
fn marks(
    p: &Painter,
    theme: &Theme,
    draft: &Draft,
    piece: PieceKey,
    state: &State,
    over: Option<PointKey>,
) {
    let doc = draft.doc();
    for &(key, at) in draft.points_cm(piece) {
        let (chosen, under) = (state.selection == Some(key), over == Some(key));
        let screen = state.view.to_screen(at);
        if chosen {
            p.circle_filled(screen, 5.0, theme.alert);
            p.circle_stroke(screen, 9.0, Stroke::new(1.0, theme.alert));
        } else if under {
            p.circle_filled(screen, 4.0, theme.ink);
        } else {
            p.circle_filled(screen, 3.0, theme.accent);
        }
        if !state.labels {
            continue;
        }
        // A name its author wrote belongs to the drawing; the automatic number
        // is only an answer to the pointer, or to a node marked to show one.
        let held = doc.points.get(key);
        let asked = held.is_some_and(|point| point.label_visible);
        let name = match held.and_then(|point| point.label.clone()) {
            Some(written) => written,
            None if asked || chosen || under => doc.label_of(piece, key).unwrap_or_default(),
            None => continue,
        };
        let font = FontId::monospace(10.0);
        let at = screen + vec2(9.0, -9.0);
        p.text(at, Align2::LEFT_BOTTOM, name, font, theme.ink_soft);
    }
}

/// What the piece on the table is called, or that there is none.
fn name_of(draft: Option<&Draft>, piece: Option<PieceKey>) -> String {
    match (draft, piece) {
        (Some(draft), Some(piece)) => draft
            .doc()
            .pieces
            .get(piece)
            .map_or_else(|| "mesa vacía".to_owned(), |held| held.name.clone()),
        _ => "mesa vacía".to_owned(),
    }
}

/// The mono line over the mat: what is drawn, at what zoom, in what unit.
fn caption(p: &Painter, theme: &Theme, rect: Rect, name: &str, zoom: f64) {
    let at = rect.translate(vec2(22.0, 20.0));
    canvas_label(p, theme, at, &format!("PATRÓN — {name} · {zoom:.0} % · cm"));
}

/// The view chips, right to left along the top of the mat.
fn chips(ui: &mut egui::Ui, theme: &Theme, rect: Rect, state: &mut State) {
    let bar = Rect::from_min_max(
        pos2(rect.left(), rect.top() + 28.0),
        pos2(rect.right() - PAD, rect.top() + 54.0),
    );
    let layout = egui::Layout::right_to_left(egui::Align::Min);
    ui.scope_builder(egui::UiBuilder::new().max_rect(bar).layout(layout), |ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if button_secondary(ui, theme, "100%").clicked() {
            state.view.one_to_one(ui.ctx().pixels_per_point());
        }
        if button_secondary(ui, theme, "Encuadrar").clicked() {
            state.frame = true;
        }
        let tag = |p: &Painter, r: Rect, c| glyph::paint(p, r, c, TAG);
        if button_icon(ui, theme, "Etiquetas", state.labels, tag).clicked() {
            state.labels = !state.labels;
        }
    });
}
