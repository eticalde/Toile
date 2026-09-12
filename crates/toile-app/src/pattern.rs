use eframe::egui;
use toile_engine::draft::{self, Binding, Command, Draft, PieceKey, PointKey};
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
/// document is written on every frame of the drag: reading the binding back
/// each frame would measure the gesture from the frame before it rather than
/// from the grab.
pub struct Drag {
    /// The node in hand, and the mark that says so.
    point: PointKey,
    /// What its two coordinates were bound to when it was grabbed.
    origin: [Binding; 2],
    /// Where it resolved to then.
    from_cm: [f64; 2],
    /// Where inside the mark the grab landed, in screen points.
    offset: egui::Vec2,
}

impl Drag {
    /// The edit one frame of the drag makes, with the node taken to `at_cm`.
    fn moved_to(&self, at_cm: [f64; 2]) -> Command {
        let to = [0, 1].map(|k| {
            bind::placed(
                &self.origin[k],
                at_cm[k],
                at_cm[k] - self.from_cm[k],
                STEP_CM,
            )
        });
        Command::MovePoint {
            point: self.point,
            to,
        }
    }
}

/// A node of the drafted piece as this panel holds it: what a command names,
/// and where the pointer finds it.
///
/// The key and the two places are made together out of one entry of the node
/// list and never travel apart, which is the whole of the guarantee that a
/// press moves the node it landed on. The drawn line is a different and longer
/// sequence — every bent tract puts curve samples between two nodes — so no
/// position in it may be used to name a node.
///
/// The pattern place says its unit in its name: this panel is the one place
/// the mesher's metres and the document's centimetres meet as the same
/// `[f64; 2]`, and only `screen` is told apart by its type.
struct Node {
    /// The node itself.
    point: PointKey,
    /// Where it resolved to.
    at_cm: [f64; 2],
    /// The same place in panel points.
    screen: egui::Pos2,
}

/// Draws the 2D pattern and applies drags straight to the session.
///
/// Every drag frame recompiles the rest state, so the 3D panel is already
/// showing the edit by the time the pointer moves again. A session with no
/// document behind it draws its contour and nothing more: there is no node to
/// name in a command, and this panel writes no other kind of edit — so it
/// paints no grabbable mark either.
pub fn show(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    theme: &Theme,
    session: &mut Session,
    drag: &mut Option<Drag>,
) {
    let (resp, painter) = widgets::mat_canvas(ui, theme, size);
    let rect = resp.rect;

    // The line is the flattening, curves and all: it is what the cloth is cut
    // along. The dots below are the nodes, which are fewer.
    let contour_m: Vec<[f64; 2]> = session.contour_m().to_vec();
    let view = DrapeView::fit(&contour_m, rect);
    let line: Vec<egui::Pos2> = contour_m
        .iter()
        .map(|&p| view.to_screen_from_m(p))
        .collect();
    painter.add(egui::Shape::closed_line(
        line,
        egui::Stroke::new(1.6, theme.outline),
    ));
    let nodes = match (session.draft(), session.piece()) {
        (Some(draft), Some(piece)) => nodes_of(draft, piece, &view),
        _ => Vec::new(),
    };

    if resp.drag_started()
        && let Some(pos) = resp.interact_pointer_pos()
    {
        *drag =
            nearest(&nodes, pos).and_then(|node| grab(session.draft()?, node, node.screen - pos));
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
        let at_cm = view.to_document(pos + held.offset);
        let _ = session.edit(held.moved_to(at_cm));
    }

    for node in &nodes {
        let (r, color) = if drag.as_ref().is_some_and(|held| held.point == node.point) {
            (5.0, theme.alert)
        } else {
            (2.4, theme.accent)
        };
        painter.circle_filled(node.screen, r, color);
    }
    let caption = if nodes.is_empty() {
        "PATRÓN 2D — el contorno que se drapea"
    } else {
        "PATRÓN 2D — arrastra un nodo"
    };
    widgets::canvas_label(&painter, theme, rect, caption);
}

/// The piece's nodes on the glass, in contour order.
///
/// Only the nodes. The samples the flattening puts along a bent tract are
/// places on a line, not entities of the document, and there is no command
/// that moves one; painting them as grabbable dots is what let a press on a
/// curve write a move of some other node entirely.
fn nodes_of(draft: &Draft, piece: PieceKey, view: &DrapeView) -> Vec<Node> {
    draft
        .points_cm(piece)
        .iter()
        .map(|&(point, at_cm)| Node {
            point,
            at_cm,
            screen: view.to_screen_from_m(draft::to_metres(at_cm)),
        })
        .collect()
}

/// The node a press lands on, with what it was bound to at that moment.
///
/// `None` when the document has since lost the node, which leaves the press
/// holding nothing rather than holding a neighbour.
fn grab(draft: &Draft, node: &Node, offset: egui::Vec2) -> Option<Drag> {
    let held = draft.doc().points.get(node.point)?;
    Some(Drag {
        point: node.point,
        origin: [held.x.clone(), held.y.clone()],
        from_cm: node.at_cm,
        offset,
    })
}

/// The node under the pointer, if one is within reach of it.
fn nearest(nodes: &[Node], pos: egui::Pos2) -> Option<&Node> {
    let mut best: (f32, Option<&Node>) = (GRAB_RADIUS, None);
    for node in nodes {
        let d = node.screen.distance(pos);
        if d < best.0 {
            best = (d, Some(node));
        }
    }
    best.1
}

/// Maps pattern metres, y up, onto panel points, y down.
///
/// Named apart from the drafting table's own `View`, which maps centimetres
/// with y down: one crate, one word, and a `to_screen`/`to_document` pair of
/// the same shape on both, so a reader carrying one file's habit into the
/// other reads the wrong unit out of an identical call.
struct DrapeView {
    centre: egui::Pos2,
    origin: (f64, f64),
    scale: f64,
}

impl DrapeView {
    fn fit(contour_m: &[[f64; 2]], rect: egui::Rect) -> Self {
        let (mut lo, mut hi) = ([f64::MAX; 2], [f64::MIN; 2]);
        for p in contour_m {
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

    /// Where a place on the piece, in the mesher's metres, lands on the glass.
    ///
    /// The unit is in the method name and not only in the parameter, because
    /// this and [`DrapeView::to_document`] read as an inverse pair and are not
    /// one: metres go in, centimetres come out. The drafting table's own `View`
    /// carries a `to_screen`/`to_document` pair of exactly this shape that is
    /// centimetres on both sides, and a call site shows neither parameter name.
    fn to_screen_from_m(&self, metres: [f64; 2]) -> egui::Pos2 {
        egui::pos2(
            self.centre.x + ((metres[0] - self.origin.0) * self.scale) as f32,
            self.centre.y - ((metres[1] - self.origin.1) * self.scale) as f32,
        )
    }

    /// Where a panel point lands on the piece, in the centimetres a command is
    /// written in.
    ///
    /// The unit conversion is inside the inverse rather than a step beside it.
    /// This panel maps the pattern in the mesher's metres and writes its edits
    /// in the document's centimetres, and both are `[f64; 2]`: a separate
    /// `draft::to_document` here was a call that could be left out and still
    /// compile, rewriting a node bound at 25.5 cm to 0.3 and autosaving it.
    fn to_document(&self, q: egui::Pos2) -> [f64; 2] {
        draft::to_document([
            self.origin.0 + f64::from(q.x - self.centre.x) / self.scale,
            self.origin.1 - f64::from(q.y - self.centre.y) / self.scale,
        ])
    }
}

#[cfg(test)]
mod tests;
