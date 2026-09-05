use eframe::egui;
use toile_engine::draft::{self, Binding, Command, PointKey};
use toile_engine::session::Session;

use crate::theme::Theme;
use crate::{bind, widgets};

/// Pixel radius within which a click grabs a control point.
const GRAB_RADIUS: f32 = 14.0;

/// Margin between the pattern's bounding box and the panel edge, in points.
const MARGIN: f64 = 40.0;

/// The resolution this panel writes at, in centimetres: it carries no snap of
/// its own, and a tenth is what the drafting table falls back to without one.
const STEP_CM: f64 = 0.1;

/// The name a drag leaves in the undo stack.
const MOVE: &str = "mover punto";

/// A node of the drafted piece on its way somewhere.
///
/// It carries what the node was bound to when it was grabbed, because the
/// document is written on every frame of the drag: a coordinate written as a
/// formula keeps its formula, and the resolved value is never written back
/// over it.
pub struct Drag {
    /// Where the node sits in the contour, for the mark that says it is held.
    index: usize,
    /// The node in hand.
    point: PointKey,
    /// What its two coordinates were bound to when it was grabbed.
    origin: [Binding; 2],
    /// Where it resolved to then, in centimetres.
    from: [f64; 2],
    /// Where inside the mark the grab landed, in screen points.
    offset: egui::Vec2,
}

impl Drag {
    /// The edit one frame of the drag makes, with the node taken to `at`.
    fn moved_to(&self, at: [f64; 2]) -> Command {
        let to =
            [0, 1].map(|k| bind::placed(&self.origin[k], at[k], at[k] - self.from[k], STEP_CM));
        Command::MovePoint {
            point: self.point,
            to,
        }
    }
}

/// Draws the 2D pattern and applies drags straight to the session.
///
/// Every drag frame recompiles the rest state, so the 3D panel is already
/// showing the edit by the time the pointer moves again. A session with no
/// document behind it draws its contour and nothing more: there is no node to
/// name in a command, and this panel writes no other kind of edit.
pub fn show(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    theme: &Theme,
    session: &mut Session,
    drag: &mut Option<Drag>,
) {
    let (resp, painter) = widgets::mat_canvas(ui, theme, size);
    let rect = resp.rect;

    let contour: Vec<[f64; 2]> = session.contour().to_vec();
    let view = View::fit(&contour, rect);
    let pts: Vec<egui::Pos2> = contour.iter().map(|&p| view.to_screen(p)).collect();
    painter.add(egui::Shape::closed_line(
        pts.clone(),
        egui::Stroke::new(1.6, theme.outline),
    ));

    if resp.drag_started()
        && let Some(pos) = resp.interact_pointer_pos()
    {
        *drag = nearest(&pts, pos).and_then(|i| grab(session, i, pts[i] - pos));
        if drag.is_some() {
            // One drag, one entry: the frames in between fold into it.
            session.begin_gesture(MOVE);
        }
    }
    if resp.drag_stopped() {
        *drag = None;
        session.end_gesture();
    }
    if let Some(held) = drag.as_ref()
        && let Some(pos) = resp.interact_pointer_pos()
    {
        let at = draft::to_document(view.to_pattern(pos + held.offset));
        let _ = session.edit(held.moved_to(at));
    }

    for (i, q) in pts.iter().enumerate() {
        let (r, color) = if drag.as_ref().is_some_and(|held| held.index == i) {
            (5.0, theme.alert)
        } else {
            (2.4, theme.accent)
        };
        painter.circle_filled(*q, r, color);
    }
    widgets::canvas_label(
        &painter,
        theme,
        rect,
        "PATRÓN 2D — arrastra un punto · las costuras en azul",
    );
}

/// The node a press lands on, with what it was bound to at that moment.
///
/// The index is a position in the contour, which is what this panel draws;
/// while every tract is a straight line that is the same thing as the node at
/// that position.
fn grab(session: &Session, index: usize, offset: egui::Vec2) -> Option<Drag> {
    let draft = session.draft()?;
    let &(point, from) = draft.points_cm(session.piece()?).get(index)?;
    let held = draft.doc().points.get(point)?;
    Some(Drag {
        index,
        point,
        origin: [held.x.clone(), held.y.clone()],
        from,
        offset,
    })
}

fn nearest(pts: &[egui::Pos2], pos: egui::Pos2) -> Option<usize> {
    let mut best = (GRAB_RADIUS, None);
    for (i, q) in pts.iter().enumerate() {
        let d = q.distance(pos);
        if d < best.0 {
            best = (d, Some(i));
        }
    }
    best.1
}

/// Maps pattern metres, y up, onto panel points, y down.
struct View {
    centre: egui::Pos2,
    origin: (f64, f64),
    scale: f64,
}

impl View {
    fn fit(contour: &[[f64; 2]], rect: egui::Rect) -> Self {
        let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
        for p in contour {
            for k in 0..2 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        let span = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1.0e-6);
        Self {
            centre: rect.center(),
            origin: (f64::midpoint(lo[0], hi[0]), f64::midpoint(lo[1], hi[1])),
            scale: (f64::from(rect.width().min(rect.height())) - 2.0 * MARGIN) / span,
        }
    }

    fn to_screen(&self, p: [f64; 2]) -> egui::Pos2 {
        egui::pos2(
            self.centre.x + ((p[0] - self.origin.0) * self.scale) as f32,
            self.centre.y - ((p[1] - self.origin.1) * self.scale) as f32,
        )
    }

    fn to_pattern(&self, q: egui::Pos2) -> [f64; 2] {
        [
            self.origin.0 + f64::from(q.x - self.centre.x) / self.scale,
            self.origin.1 - f64::from(q.y - self.centre.y) / self.scale,
        ]
    }
}
