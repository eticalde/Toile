use toile_engine::draft::PointKey;

use super::tract::{self, Tract};

/// How near the pointer has to come to catch each thing, in screen points.
///
/// The budgets are on the glass and not in the pattern, which is what makes
/// zooming in the way to aim at a crowded corner.
pub const NODE_PT: f64 = 9.0;
pub const EDGE_PT: f64 = 8.0;

/// The nearest place on a tract, and how far away it was.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Nearest {
    /// Where the tract's node sits in the contour.
    pub from: usize,
    /// How far along the tract, from zero at its node to one at the next.
    ///
    /// Arc length along the drawn line, which is what `EdgeAnchor.t` means:
    /// on a bent tract it is not the cubic's own parameter.
    pub t: f64,
    /// The place itself, in centimetres.
    pub at: [f64; 2],
    /// The distance to it, in centimetres.
    pub away: f64,
}

/// What the pointer is over, when it is over anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hover {
    /// The bare mat.
    None,
    /// A node of the piece.
    Node(PointKey),
    /// A handle of one of its tracts, out of the ones on show.
    Handle(PointKey),
    /// The tract leaving a node.
    Edge(PointKey),
}

/// What the pointer is over: a node, then a handle, then the tract under it.
///
/// The order is the snap ladder's, so what the pointer reports and what a
/// press takes hold of cannot disagree.
pub fn under(
    at: [f64; 2],
    nodes: &[(PointKey, [f64; 2])],
    handles: &[(PointKey, [f64; 2])],
    tracts: &[Tract],
    scale: f64,
) -> Hover {
    let reach = |budget: f64| budget / scale.max(f64::EPSILON);
    if let Some((key, _)) = nearest_node(at, nodes, &[], reach(NODE_PT)) {
        return Hover::Node(key);
    }
    if let Some((key, _)) = nearest_node(at, handles, &[], reach(NODE_PT)) {
        return Hover::Handle(key);
    }
    match tract::nearest(at, tracts, &[]) {
        Some(found) if found.away < reach(EDGE_PT) => Hover::Edge(tracts[found.from].node),
        _ => Hover::None,
    }
}

/// The node within `reach` centimetres, the nearest one when there are two.
///
/// Everything in `held` is left out: a gesture carries those, so they sit
/// where the last frame put them rather than where the document means them to
/// be, and catching one would place a point against itself.
pub fn nearest_node(
    at: [f64; 2],
    nodes: &[(PointKey, [f64; 2])],
    held: &[PointKey],
    reach: f64,
) -> Option<(PointKey, [f64; 2])> {
    let mut best: Option<(f64, PointKey, [f64; 2])> = None;
    for &(key, place) in nodes {
        if held.contains(&key) {
            continue;
        }
        let away = away(place, at);
        if away < reach && best.is_none_or(|(held, _, _)| away < held) {
            best = Some((away, key, place));
        }
    }
    best.map(|(_, key, place)| (key, place))
}

/// The distance between two places, in the units they are given in.
pub fn away(a: [f64; 2], b: [f64; 2]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// The nearest place to `q` on the segment `a → b`, clamped to its ends.
pub fn nearest_on(a: [f64; 2], b: [f64; 2], q: [f64; 2], from: usize) -> Nearest {
    let run = [b[0] - a[0], b[1] - a[1]];
    let span = run[0] * run[0] + run[1] * run[1];
    let t = if span <= f64::EPSILON {
        0.0
    } else {
        (((q[0] - a[0]) * run[0] + (q[1] - a[1]) * run[1]) / span).clamp(0.0, 1.0)
    };
    let at = [a[0] + run[0] * t, a[1] + run[1] * t];
    Nearest {
        from,
        t,
        at,
        away: away(at, q),
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "a place clamped to the end of a segment is that end exactly"
    )]

    use super::*;

    /// A square, in contour order, with a node key per corner.
    fn square() -> Vec<(PointKey, [f64; 2])> {
        [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]
            .into_iter()
            .enumerate()
            .map(|(i, at)| (PointKey::new(i as u32, 0), at))
            .collect()
    }

    #[test]
    fn nearest_on_a_segment_clamps_to_its_ends() {
        let past = nearest_on([0.0, 0.0], [10.0, 0.0], [14.0, 3.0], 0);
        assert_eq!(past.t, 1.0);
        assert_eq!(past.at, [10.0, 0.0]);
        assert!((past.away - 5.0).abs() < 1.0e-9);
        let before = nearest_on([0.0, 0.0], [10.0, 0.0], [-4.0, 0.0], 0);
        assert_eq!(before.t, 0.0);
        assert_eq!(before.at, [0.0, 0.0]);
    }

    #[test]
    fn a_degenerate_tract_reports_its_own_node() {
        let doubled = [[4.0, 4.0], [4.0, 4.0]];
        let found = nearest_on(doubled[0], doubled[1], [5.0, 4.0], 0);
        assert_eq!(found.t, 0.0);
        assert!((found.away - 1.0).abs() < 1.0e-9);
    }

    #[test]
    fn nothing_in_hand_is_a_candidate() {
        let nodes = square();
        assert!(nearest_node([0.2, 0.2], &nodes, &[nodes[0].0], 1.0).is_none());
        assert!(nearest_node([0.2, 0.2], &nodes, &[], 1.0).is_some());
        // The whole hand, not only the point the pointer took hold of: a
        // gesture carries its selection and a node's handles with it.
        let hand = [nodes[2].0, nodes[0].0];
        assert!(nearest_node([0.2, 0.2], &nodes, &hand, 1.0).is_none());
    }

    #[test]
    fn a_node_beats_the_handle_and_the_handle_beats_the_tract() {
        let nodes = square();
        let tracts = tract::straight(&nodes);
        let handles = [(PointKey::new(9, 0), [5.0, 0.05])];
        let over = |at| under(at, &nodes, &handles, &tracts, 40.0);
        assert_eq!(over([10.2, 0.1]), Hover::Node(nodes[1].0));
        assert_eq!(over([5.0, 0.1]), Hover::Handle(handles[0].0));
        assert_eq!(over([2.0, 0.1]), Hover::Edge(nodes[0].0));
        assert_eq!(over([5.0, 5.0]), Hover::None);
    }
}
