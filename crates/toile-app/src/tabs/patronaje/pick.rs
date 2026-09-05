use toile_engine::draft::PointKey;

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
    /// The tract leaving a node.
    Edge(PointKey),
}

/// What the pointer is over: a node first, then the tract under it.
pub fn under(at: [f64; 2], nodes: &[(PointKey, [f64; 2])], scale: f64) -> Hover {
    let reach = |budget: f64| budget / scale.max(f64::EPSILON);
    if let Some((key, _)) = nearest_node(at, nodes, None, reach(NODE_PT)) {
        return Hover::Node(key);
    }
    match nearest_edge(at, nodes, None) {
        Some(found) if found.away < reach(EDGE_PT) => Hover::Edge(nodes[found.from].0),
        _ => Hover::None,
    }
}

/// The node within `reach` centimetres, the nearest one when there are two.
pub fn nearest_node(
    at: [f64; 2],
    nodes: &[(PointKey, [f64; 2])],
    held: Option<PointKey>,
    reach: f64,
) -> Option<(PointKey, [f64; 2])> {
    let mut best: Option<(f64, PointKey, [f64; 2])> = None;
    for &(key, place) in nodes {
        if held == Some(key) {
            continue;
        }
        let away = away(place, at);
        if away < reach && best.is_none_or(|(held, _, _)| away < held) {
            best = Some((away, key, place));
        }
    }
    best.map(|(_, key, place)| (key, place))
}

/// The nearest place on any tract of the contour.
///
/// The tracts that touch `held` are left out: a node cannot be placed against
/// the very lines it is dragging along with it.
pub fn nearest_edge(
    at: [f64; 2],
    nodes: &[(PointKey, [f64; 2])],
    held: Option<PointKey>,
) -> Option<Nearest> {
    let n = nodes.len();
    if n < 2 {
        return None;
    }
    let mut best: Option<Nearest> = None;
    for from in 0..n {
        let (head, a) = nodes[from];
        let (tail, b) = nodes[(from + 1) % n];
        if held == Some(head) || held == Some(tail) {
            continue;
        }
        let found = nearest_on(a, b, at, from);
        if best.is_none_or(|kept| found.away < kept.away) {
            best = Some(found);
        }
    }
    best
}

/// The distance between two places, in the units they are given in.
pub fn away(a: [f64; 2], b: [f64; 2]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// The nearest place to `q` on the segment `a → b`, clamped to its ends.
fn nearest_on(a: [f64; 2], b: [f64; 2], q: [f64; 2], from: usize) -> Nearest {
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
    fn an_edge_hit_reports_its_local_fraction() {
        let found = nearest_edge([2.5, 0.3], &square(), None).expect("the square has tracts");
        assert_eq!(found.from, 0);
        assert!((found.t - 0.25).abs() < 1.0e-9, "{found:?}");
        assert!((found.away - 0.3).abs() < 1.0e-9, "{found:?}");
    }

    #[test]
    fn a_degenerate_tract_reports_its_own_node() {
        let doubled = [
            (PointKey::new(0, 0), [4.0, 4.0]),
            (PointKey::new(1, 0), [4.0, 4.0]),
        ];
        let found = nearest_edge([5.0, 4.0], &doubled, None).expect("two nodes make a tract");
        assert_eq!(found.t, 0.0);
        assert!((found.away - 1.0).abs() < 1.0e-9);
    }

    #[test]
    fn the_node_in_hand_hides_the_tracts_that_hang_from_it() {
        let nodes = square();
        let held = Some(nodes[0].0);
        let found = nearest_edge([0.2, 0.2], &nodes, held).expect("two tracts are left");
        assert!(found.from == 1 || found.from == 2, "{found:?}");
        assert!(nearest_node([0.2, 0.2], &nodes, held, 1.0).is_none());
    }

    #[test]
    fn a_node_beats_the_tract_it_sits_on() {
        let nodes = square();
        assert_eq!(under([10.2, 0.1], &nodes, 40.0), Hover::Node(nodes[1].0));
        assert_eq!(under([5.0, 0.1], &nodes, 40.0), Hover::Edge(nodes[0].0));
        assert_eq!(under([5.0, 5.0], &nodes, 40.0), Hover::None);
    }
}
