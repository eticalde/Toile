use eframe::egui::{self, Color32, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, vec2};
use toile_engine::draft::{Draft, PieceKey, PointKey};

use super::curve::Bend;
use super::gesture::{self, Gesture};
use super::state::{Scope, State};
use super::tract::Tract;
use super::view::{self, View};
use super::wire::{self, Verb};
use super::{
    caption, curve, dimension, empty, marks, overview, paper, pick, precision, ruler, snap, tract,
};
use crate::theme::Theme;
use crate::widgets::{fill, grid};

/// The closest the mat draws its lines; under that they read as noise.
const GRID_MIN: f32 = 9.0;

/// The cutting mat: the whole product or one piece of it, at whatever scale
/// the view holds, with its rulers and the line over it that says which.
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
            let mut verbs = Vec::new();
            let mat = (&resp, &painter);
            match draft {
                Some(draft) if state.scope == Scope::Product => {
                    overview::show(ui, theme, mat, draft, state, &mut verbs);
                }
                _ => detail(ui, theme, mat, draft, piece, state, &mut verbs),
            }
            ruler::show(&painter, theme, rect, state.view);
            caption::show(ui, theme, rect, draft, piece, state);
            // The way onto the table shows only with no document at all. A
            // blank document is an empty mat ready to draw, not the splash.
            if draft.is_none() {
                state.asked = empty::show(ui, theme, rect).or(state.asked);
            }
            wire::answer(ui, theme, rect, state, &mut verbs);
            verbs
        })
        .inner
}

/// One piece alone, in its own coordinates: the mat that drafts, every tool
/// in hand, and the marks the pointer is making on it.
fn detail(
    ui: &mut egui::Ui,
    theme: &Theme,
    (resp, painter): (&Response, &Painter),
    draft: Option<&Draft>,
    piece: Option<PieceKey>,
    state: &mut State,
    verbs: &mut Vec<Verb>,
) {
    let rect = resp.rect;
    let nodes: &[(PointKey, [f64; 2])] = match (draft, piece) {
        (Some(draft), Some(piece)) => draft.points_cm(piece),
        _ => &[],
    };
    let drawing = draft.zip(piece);
    let (tracts, bends) = drawn(drawing);
    frame_once(state, view::bounds(nodes), rect);
    if state.ask.is_none() {
        wire::view_keys(ui, resp, state);
        if let Some(draft) = draft {
            // With no piece in front — a product with none drawn yet — the mat
            // still takes a drawing: the target is the key the next piece will
            // be given, which is the one the draw gesture goes on to create.
            // Every other tool needs geometry, and there is none, so they fall
            // through to nothing.
            let target = piece.unwrap_or_else(|| PieceKey::new(draft.doc().pieces.issued(), 0));
            let table = wire::Table {
                doc: draft.doc(),
                piece: target,
                nodes,
                tracts: &tracts,
                bends: &bends,
            };
            wire::reduce(ui, resp, &table, state, verbs);
        }
    }
    let shown = curve::handles(&bends, &state.selection);
    let over = resp.hover_pos().map_or(pick::Hover::None, |at| {
        let cm = state.view.to_document(at);
        pick::under(cm, nodes, &shown, &tracts, state.view.scale())
    });
    fill(painter, theme, rect);
    mat_grid(painter, theme, rect, state.view);
    origin(painter, theme, rect, state.view);
    if let Some((draft, piece)) = drawing {
        let ink = if draft.defects(piece).is_empty() {
            theme.outline
        } else {
            theme.alert
        };
        let line = Stroke::new(1.5, ink);
        paper_and_outline(
            painter,
            draft.flat_cm(piece),
            state.view,
            &[theme.paper],
            line,
        );
        dimension::show(painter, theme, draft, piece, state, over);
        marks::bends(painter, theme, &bends, state, over);
        marks::nodes(painter, theme, draft, piece, state, over);
    }
    match &state.gesture {
        Gesture::Drag(drag) => {
            if let Some(snapped) = state.caught {
                marks::candidate(painter, theme, state.view, snapped, drag.anchor().from);
            }
            precision::show(painter, theme, state.view, drag);
        }
        Gesture::Marquee { from, to } => {
            marks::band(painter, theme, gesture::band(state.view, *from, *to));
        }
        Gesture::Drawing {
            pending, rubber, ..
        } => {
            if let Some(snapped) = state.caught {
                let anchor = pending.last().copied().unwrap_or(*rubber);
                marks::candidate(painter, theme, state.view, snapped, anchor);
            }
            marks::drawing(painter, theme, state.view, pending, *rubber);
        }
        Gesture::Idle | Gesture::Pan { .. } | Gesture::Arrange(_) => {}
    }
}

/// The piece's tracts and its bends, both empty when the table is.
fn drawn(drawing: Option<(&Draft, PieceKey)>) -> (Vec<Tract>, Vec<Bend>) {
    match drawing {
        Some((draft, piece)) => (tract::of(draft, piece), curve::bends(draft, piece)),
        None => (Vec::new(), Vec::new()),
    }
}

/// Frames a box of the document on the first frame that has one to frame.
pub(super) fn frame_once(state: &mut State, bbox: Option<Rect>, rect: Rect) {
    let inner = Rect::from_min_max(
        rect.left_top() + vec2(ruler::BAND, ruler::BAND),
        rect.right_bottom(),
    );
    if state.frame
        && let Some(bbox) = bbox
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
pub(super) fn mat_grid(p: &Painter, theme: &Theme, rect: Rect, view: View) {
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

/// The origin cross: the pattern's (0, 0), drawn over the grid so the centre a
/// draft is measured from is never in doubt. Each axis shows only while it
/// falls on the mat; the label only while their crossing does.
fn origin(p: &Painter, theme: &Theme, rect: Rect, view: View) {
    let o = view.to_screen([0.0, 0.0]);
    let stroke = Stroke::new(1.0, theme.accent.gamma_multiply(0.55));
    if o.x >= rect.left() && o.x <= rect.right() {
        p.vline(o.x, rect.y_range(), stroke);
    }
    if o.y >= rect.top() && o.y <= rect.bottom() {
        p.hline(rect.x_range(), o.y, stroke);
    }
    if rect.contains(o) {
        p.text(
            o + vec2(4.0, 3.0),
            egui::Align2::LEFT_TOP,
            "0,0",
            egui::FontId::monospace(10.0),
            theme.muted,
        );
    }
}

/// A piece itself: paper under an outline, both drawn from the flattening and
/// not from the nodes.
///
/// A bent tract is painted as the line it will be cut along rather than as
/// the chord under it. Drawing the true cubic instead would look smoother and
/// lie: the polyline is what the mesher and the export take. Each ground is
/// laid over the one before, so a tint lies on the paper instead of hiding it.
pub(super) fn paper_and_outline(
    p: &Painter,
    outline: &[[f64; 2]],
    view: View,
    grounds: &[Color32],
    ink: Stroke,
) {
    if outline.len() < 3 {
        return;
    }
    for &ground in grounds {
        p.add(paper::sheet(outline, view, ground));
    }
    let line: Vec<Pos2> = outline.iter().map(|&at| view.to_screen(at)).collect();
    p.add(Shape::closed_line(line, ink));
}
