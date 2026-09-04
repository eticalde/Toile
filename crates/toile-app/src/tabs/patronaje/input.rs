use eframe::egui::{Pos2, Rect, pos2};
use toile_engine::draft::PointKey;

use super::view::View;

/// How near the pointer has to land to catch a node, in screen points.
///
/// The reach is on the glass rather than in the pattern, which is what makes
/// zooming in the way to aim at a crowded corner.
const GRAB: f32 = 10.0;

/// The node a click lands on, if it lands on one.
///
/// Nothing within reach means the mat itself was clicked, which is the way out
/// of a selection.
pub fn pick(nodes: &[(PointKey, [f64; 2])], view: View, at: Pos2) -> Option<PointKey> {
    let mut best = (GRAB, None);
    for &(key, cm) in nodes {
        let reach = view.to_screen(cm).distance(at);
        if reach < best.0 {
            best = (reach, Some(key));
        }
    }
    best.1
}

/// The document-space box a piece's nodes occupy, in centimetres.
///
/// Empty when the piece has none, which is what a framing of nothing is worth.
pub fn bounds(nodes: &[(PointKey, [f64; 2])]) -> Option<Rect> {
    let (first, _) = nodes.split_first()?;
    let mut bbox = Rect::from_min_max(place(first.1), place(first.1));
    for &(_, cm) in nodes {
        bbox.extend_with(place(cm));
    }
    Some(bbox)
}

/// One document point as the geometry helpers want it.
fn place(cm: [f64; 2]) -> Pos2 {
    pos2(cm[0] as f32, cm[1] as f32)
}

#[cfg(test)]
mod tests {
    use eframe::egui::vec2;

    use super::*;

    fn nodes() -> Vec<(PointKey, [f64; 2])> {
        [[0.0, 0.0], [22.0, 0.0], [25.5, 20.0]]
            .into_iter()
            .enumerate()
            .map(|(i, cm)| (PointKey::new(i as u32, 0), cm))
            .collect()
    }

    #[test]
    fn a_click_on_a_point_selects_it() {
        let view = View::default();
        let nodes = nodes();
        let at = view.to_screen(nodes[1].1);
        assert_eq!(pick(&nodes, view, at + vec2(2.0, 2.0)), Some(nodes[1].0));
    }

    #[test]
    fn a_click_on_the_mat_clears_the_selection() {
        let view = View::default();
        let nodes = nodes();
        let away = view.to_screen([60.0, 60.0]);
        assert_eq!(pick(&nodes, view, away), None);
    }

    #[test]
    fn the_nearer_of_two_nodes_wins() {
        let view = View::default();
        let nodes = nodes();
        let between = view.to_screen(nodes[0].1) + vec2(4.0, 0.0);
        assert_eq!(pick(&nodes, view, between), Some(nodes[0].0));
    }

    #[test]
    fn nothing_has_no_bounds() {
        assert_eq!(bounds(&[]), None);
        let bbox = bounds(&nodes()).expect("three nodes make a box");
        assert!((bbox.width() - 25.5).abs() < 1.0e-4, "{bbox:?}");
        assert!((bbox.height() - 20.0).abs() < 1.0e-4, "{bbox:?}");
    }

    #[test]
    fn a_click_at_the_grab_edge_misses() {
        let view = View::default();
        let nodes = nodes();
        let at = view.to_screen(nodes[0].1) + vec2(GRAB + 0.5, 0.0);
        assert_eq!(pick(&nodes, view, at), None);
    }
}
