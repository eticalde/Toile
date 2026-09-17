#![allow(
    clippy::float_cmp,
    reason = "a node is grabbed at the coordinates the draft resolved it to"
)]

use eframe::egui::{Rect, pos2, vec2};
use toile_engine::draft::block;

use super::*;

/// The half of the Probador this panel gets, near enough: the numbers below
/// are read off it, so a different size would need different ones.
const PANEL: [f32; 2] = [520.0, 520.0];

/// The shipped trouser front on the glass.
///
/// Two of its nine tracts bend, which is the whole point of testing against
/// it: the line it draws is far longer than the list of nodes that can be
/// moved, and a panel that confuses the two has forty-seven places to go wrong.
struct Table {
    draft: Draft,
    piece: PieceKey,
    view: DrapeView,
    nodes: Vec<Node>,
}

fn table() -> Table {
    let draft = Draft::from_doc(block::trouser_front()).expect("the block resolves");
    let piece = draft
        .doc()
        .piece_named(block::FRONT)
        .expect("the block draws one piece");
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(PANEL[0], PANEL[1]));
    let view = DrapeView::fit(draft.outline_m(piece), rect);
    let nodes = nodes_of(&draft, piece, &view);
    Table {
        draft,
        piece,
        view,
        nodes,
    }
}

impl Table {
    /// Where a point of the drawn line falls on the glass.
    fn on_glass(&self, cm: [f64; 2]) -> egui::Pos2 {
        self.view.to_screen_from_m(draft::to_metres(cm))
    }

    /// The point of the drawn line furthest from every dot, and how far.
    ///
    /// Somewhere along a bent tract the drawing runs clear of both the node it
    /// leaves and the node it reaches. That stretch is the panel's exposure: it
    /// is line the pointer can press on and node the pointer is not on.
    fn loneliest(&self) -> (egui::Pos2, f32) {
        let mut worst = (pos2(0.0, 0.0), 0.0);
        for &cm in self.draft.flat_cm(self.piece) {
            let at = self.on_glass(cm);
            let gap = self
                .nodes
                .iter()
                .map(|node| node.screen.distance(at))
                .fold(f32::MAX, f32::min);
            if gap > worst.1 {
                worst = (at, gap);
            }
        }
        worst
    }
}

#[test]
fn the_dots_are_the_nodes_and_not_the_line_through_them() {
    let table = table();
    // Nine nodes; forty-seven points on the line they draw, because the hip is
    // flattened into twenty-four of them and the crotch into sixteen. A dot per
    // point of the line would be five times as many dots as there are nodes to
    // move, and only the first two would sit on the node of their own index.
    assert_eq!(table.nodes.len(), 9);
    assert_eq!(table.draft.flat_cm(table.piece).len(), 47);
    for (node, &(point, at_cm)) in table.nodes.iter().zip(table.draft.points_cm(table.piece)) {
        assert_eq!(node.point, point);
        assert_eq!(node.at_cm, at_cm);
        assert_eq!(node.screen, table.on_glass(at_cm));
    }
}

#[test]
fn a_press_on_a_dot_takes_the_node_that_dot_is() {
    let table = table();
    for dot in &table.nodes {
        let caught = nearest(&table.nodes, dot.screen).expect("a press on a dot catches it");
        let held = grab(&table.draft, caught, caught.screen - dot.screen)
            .expect("the node is in the document");
        assert_eq!(held.point, dot.point, "the press moved another node");
        // And the place the drag measures its delta from is where the document
        // says that node is, not where some neighbour of it is.
        assert_eq!(held.from_cm, dot.at_cm);
        assert_eq!(table.draft.resolved(held.point), Some(dot.at_cm));
    }
}

#[test]
fn a_press_anywhere_on_the_drawn_line_moves_no_node_out_of_reach() {
    let table = table();
    for &cm in table.draft.flat_cm(table.piece) {
        let at = table.on_glass(cm);
        let Some(caught) = nearest(&table.nodes, at) else {
            continue;
        };
        let held = grab(&table.draft, caught, caught.screen - at).expect("the node is live");
        let moved = table.on_glass(held.from_cm);
        assert!(
            moved.distance(at) < GRAB_RADIUS,
            "a press at {at:?} took a node at {moved:?}"
        );
    }
}

#[test]
fn a_press_in_the_middle_of_a_bent_tract_takes_nothing() {
    let table = table();
    let (at, gap) = table.loneliest();
    // Self-check: on a piece whose curves all fell inside the grab radius there
    // would be nothing here to press on that is not already a node.
    assert!(
        gap > GRAB_RADIUS,
        "the block's curves run clear of its nodes: {gap} points"
    );
    assert!(
        nearest(&table.nodes, at).is_none(),
        "a sample of a curve is a place on a line, not a node"
    );
}

#[test]
fn a_table_with_no_document_paints_nothing_to_grab() {
    let session = Session::demo_bodice();
    assert!(session.draft().is_none());
    // The demo bodice draws a contour of a hundred and twenty-eight points and
    // owns no node at all: every one of them was a dot promising a drag that
    // could never be issued.
    let contours: Vec<&[[f64; 2]]> = session.contours_m().collect();
    assert_eq!(contours.len(), 1, "the demo drapes the one panel");
    assert!(contours[0].len() > 100);
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(PANEL[0], PANEL[1]));
    let view = DrapeView::fit(contours.iter().copied().flatten(), rect);
    let nodes: Vec<Node> = match session.draft() {
        Some(draft) => session
            .pieces()
            .into_iter()
            .flat_map(|piece| nodes_of(draft, piece, &view))
            .collect(),
        None => Vec::new(),
    };
    assert!(nodes.is_empty());
}
