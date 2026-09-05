use eframe::egui::{self, Painter, Pos2, Rect, Sense, Shape, Stroke, pos2, vec2};
use toile_engine::draft::{Draft, PieceKey, PointKey};

use super::curve::Bend;
use super::gesture::{self, Gesture};
use super::state::State;
use super::tract::Tract;
use super::view::{self, View};
use super::wire::{self, Verb};
use super::{curve, dimension, empty, marks, paper, pick, precision, ruler, snap, tract};
use crate::glyph;
use crate::theme::Theme;
use crate::widgets::{PAD, button_icon, button_secondary, canvas_label, fill, grid};

/// The closest the mat draws its lines; under that they read as noise.
const GRID_MIN: f32 = 9.0;

const TAG: &str = "2 4 10 4 14 8 10 12 2 12 2 4; o 5 8 1.3";

/// The cutting mat: the piece at whatever scale the view holds, its rulers,
/// the marks the pointer is making, and the chips that move it.
pub fn show(
    ui: &mut egui::Ui,
    theme: &Theme,
    draft: Option<&Draft>,
    piece: Option<PieceKey>,
    state: &mut State,
) -> Vec<Verb> {
    egui::CentralPanel::no_frame()
        .show(ui, |ui| {
            let size = ui.available_size();
            let (resp, painter) = ui.allocate_painter(size, Sense::click_and_drag());
            let rect = resp.rect;
            let nodes: &[(PointKey, [f64; 2])] = match (draft, piece) {
                (Some(draft), Some(piece)) => draft.points_cm(piece),
                _ => &[],
            };
            let drawing = draft.zip(piece);
            let (tracts, bends) = drawn(drawing);
            frame_once(state, nodes, rect);
            let mut verbs = Vec::new();
            if state.ask.is_none() {
                wire::view_keys(ui, &resp, state);
                if let Some((draft, piece)) = drawing {
                    let table = wire::Table {
                        doc: draft.doc(),
                        piece,
                        nodes,
                        tracts: &tracts,
                        bends: &bends,
                    };
                    wire::reduce(ui, &resp, &table, state, &mut verbs);
                }
            }
            let shown = curve::handles(&bends, &state.selection);
            let over = resp.hover_pos().map_or(pick::Hover::None, |at| {
                pick::under(
                    state.view.to_document(at),
                    nodes,
                    &shown,
                    &tracts,
                    state.view.scale(),
                )
            });
            fill(&painter, theme, rect);
            mat_grid(&painter, theme, rect, state.view);
            if let Some((draft, piece)) = drawing {
                paper_and_outline(&painter, theme, draft, piece, state.view);
                dimension::show(&painter, theme, draft, piece, state, over);
                marks::bends(&painter, theme, &bends, state, over);
                marks::nodes(&painter, theme, draft, piece, state, over);
            }
            match &state.gesture {
                Gesture::Drag(drag) => {
                    if let Some(snapped) = state.caught {
                        marks::candidate(&painter, theme, state.view, snapped, drag.anchor().from);
                    }
                    precision::show(&painter, theme, state.view, drag);
                }
                Gesture::Marquee { from, to } => {
                    marks::band(&painter, theme, gesture::band(state.view, *from, *to));
                }
                Gesture::Idle | Gesture::Pan { .. } => {}
            }
            ruler::show(&painter, theme, rect, state.view);
            let zoom = state.view.zoom_percent(ui.ctx().pixels_per_point());
            caption(&painter, theme, rect, &name_of(draft, piece), zoom);
            chips(ui, theme, rect, state);
            if drawing.is_none() {
                state.asked = empty::show(ui, theme, rect).or(state.asked);
            }
            wire::answer(ui, theme, rect, state, &mut verbs);
            verbs
        })
        .inner
}

/// The piece's tracts and its bends, both empty when the table is.
fn drawn(drawing: Option<(&Draft, PieceKey)>) -> (Vec<Tract>, Vec<Bend>) {
    match drawing {
        Some((draft, piece)) => (tract::of(draft, piece), curve::bends(draft, piece)),
        None => (Vec::new(), Vec::new()),
    }
}

/// Frames the piece on the first frame that has one to frame.
fn frame_once(state: &mut State, nodes: &[(PointKey, [f64; 2])], rect: Rect) {
    let inner = Rect::from_min_max(
        rect.left_top() + vec2(ruler::BAND, ruler::BAND),
        rect.right_bottom(),
    );
    if state.frame
        && let Some(bbox) = view::bounds(nodes)
    {
        state.view.fit(bbox, inner);
        state.frame = false;
    }
}

/// The ruled lines, travelling with the view so a centimetre stays a
/// centimetre wherever the drawing has been dragged to.
///
/// The centimetre itself is drawn whenever there is room for it, so that the
/// grid on the mat is the grid the pointer catches; under that it falls back
/// to the decade the rulers are counting in.
fn mat_grid(p: &Painter, theme: &Theme, rect: Rect, view: View) {
    let fine = (snap::GRID_CM * view.scale()) as f32;
    let step = if fine >= GRID_MIN {
        fine
    } else {
        (ruler::step_cm(view.scale()) * view.scale() / 2.0) as f32
    };
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
///
/// Both are drawn from the flattening and not from the nodes, so a bent tract
/// is painted as the line it will be cut along rather than as the chord under
/// it. Drawing the true cubic instead would look smoother and lie: the
/// polyline is what the mesher and the export take.
fn paper_and_outline(p: &Painter, theme: &Theme, draft: &Draft, piece: PieceKey, view: View) {
    let cm: Vec<[f64; 2]> = draft.flat_cm(piece).to_vec();
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
