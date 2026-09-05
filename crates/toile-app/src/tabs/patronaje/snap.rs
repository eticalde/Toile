use toile_engine::draft::PointKey;

use super::pick::{self, EDGE_PT, NODE_PT};

/// How near a grid line has to fall to catch a position, in screen points.
const GRID_PT: f64 = 6.0;

/// The resolution a free position is written to, in centimetres.
const FREE_STEP: f64 = 0.1;

/// The spacing of the grid a position is caught by, in centimetres.
///
/// A centimetre, which is the unit of the trade, and not a function of the
/// zoom: the reach is what the zoom changes, so coming closer is what lets a
/// position land between two of these lines.
pub const GRID_CM: f64 = 1.0;

/// The two directions of an eighth turn.
const DIAG: f64 = core::f64::consts::FRAC_1_SQRT_2;

/// The directions an axis constraint allows, from the anchor of the gesture.
const AXES: [[f64; 2]; 4] = [[1.0, 0.0], [0.0, 1.0], [DIAG, DIAG], [DIAG, -DIAG]];

/// What caught a position.
///
/// The ladder is fixed and it is short: node, then tract, then the axis of the
/// gesture, then the drawn grid. Handles and crossings join it on the day the
/// tools that draw them do.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SnapKind {
    /// An existing node of the piece.
    Node(PointKey),
    /// A place along the tract leaving a node, at a fraction of its length.
    Edge { from: PointKey, t: f64 },
    /// The horizontal, vertical or diagonal through the anchor.
    Axis,
    /// A line of the grid the mat draws.
    Grid,
}

/// Where a position ends up, and what caught it there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Snapped {
    /// The position, in centimetres.
    pub at: [f64; 2],
    /// What caught it, when anything did.
    pub kind: Option<SnapKind>,
}

/// What the pointer is allowed to catch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapConfig {
    /// The spacing of the drawn grid, in centimetres.
    pub grid_cm: f64,
    /// Whether anything is caught at all: `Ctrl` puts this out while held.
    pub on: bool,
    /// Whether the position is held to an axis of the anchor: `Shift`.
    pub axis: bool,
}

impl Default for SnapConfig {
    fn default() -> SnapConfig {
        SnapConfig {
            grid_cm: GRID_CM,
            on: true,
            axis: false,
        }
    }
}

impl SnapConfig {
    /// The resolution a nudged formula is written to, in centimetres.
    ///
    /// A drag that snaps writes whole units of what it snaps to; one that does
    /// not still rounds, because a coordinate carrying the rounding noise of a
    /// drag out to the fifteenth decimal is not something anyone can read.
    pub fn step_cm(self) -> f64 {
        if self.on { self.grid_cm } else { FREE_STEP }
    }
}

/// What the ladder has to look at.
pub struct SnapContext<'a> {
    /// The piece's nodes, resolved, in contour order, in centimetres.
    pub nodes: &'a [(PointKey, [f64; 2])],
    /// The node in hand, which catches neither itself nor its own tracts.
    pub held: Option<PointKey>,
    /// Where the gesture started, in centimetres: the axes run through it.
    pub anchor: [f64; 2],
    /// Screen points per centimetre.
    pub scale: f64,
}

/// Runs the ladder: the first candidate inside its budget wins.
pub fn resolve(raw: [f64; 2], ctx: &SnapContext<'_>, cfg: SnapConfig) -> Snapped {
    let at = if cfg.axis {
        constrain(raw, ctx.anchor)
    } else {
        raw
    };
    let axis = cfg.axis.then_some(SnapKind::Axis);
    if !cfg.on {
        return Snapped { at, kind: axis };
    }
    let reach = |budget: f64| budget / ctx.scale.max(f64::EPSILON);
    if let Some((key, place)) = pick::nearest_node(at, ctx.nodes, ctx.held, reach(NODE_PT)) {
        return Snapped {
            at: place,
            kind: Some(SnapKind::Node(key)),
        };
    }
    if let Some(found) = pick::nearest_edge(at, ctx.nodes, ctx.held)
        && found.away < reach(EDGE_PT)
    {
        return Snapped {
            at: found.at,
            kind: Some(SnapKind::Edge {
                from: ctx.nodes[found.from].0,
                t: found.t,
            }),
        };
    }
    if axis.is_some() {
        return Snapped { at, kind: axis };
    }
    grid(at, cfg.grid_cm, reach(GRID_PT)).unwrap_or(Snapped { at, kind: None })
}

/// The grid line each coordinate falls against, when it falls near one.
fn grid(at: [f64; 2], step: f64, reach: f64) -> Option<Snapped> {
    if step <= 0.0 || !step.is_finite() {
        return None;
    }
    let mut out = at;
    let mut caught = false;
    for k in 0..2 {
        let ruled = (at[k] / step).round() * step;
        if (ruled - at[k]).abs() < reach {
            out[k] = ruled;
            caught = true;
        }
    }
    caught.then_some(Snapped {
        at: out,
        kind: Some(SnapKind::Grid),
    })
}

/// The position held to whichever eighth turn from the anchor it is nearest.
fn constrain(raw: [f64; 2], anchor: [f64; 2]) -> [f64; 2] {
    let away = [raw[0] - anchor[0], raw[1] - anchor[1]];
    let mut best = (f64::NEG_INFINITY, [0.0, 0.0]);
    for dir in AXES {
        let along = away[0] * dir[0] + away[1] * dir[1];
        if along.abs() > best.0 {
            best = (along.abs(), [dir[0] * along, dir[1] * along]);
        }
    }
    [anchor[0] + best.1[0], anchor[1] + best.1[1]]
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "a position that snapped is the candidate's own, to the bit"
    )]

    use super::*;

    /// A square, in contour order, at a scale where a screen point is a tenth
    /// of a centimetre.
    const SCALE: f64 = 10.0;

    fn square() -> Vec<(PointKey, [f64; 2])> {
        [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]
            .into_iter()
            .enumerate()
            .map(|(i, at)| (PointKey::new(i as u32, 0), at))
            .collect()
    }

    fn context(nodes: &[(PointKey, [f64; 2])], scale: f64) -> SnapContext<'_> {
        SnapContext {
            nodes,
            held: None,
            anchor: [0.0, 0.0],
            scale,
        }
    }

    #[test]
    fn a_node_wins_over_the_grid_within_the_budget() {
        let nodes = square();
        let caught = resolve([10.3, 0.2], &context(&nodes, SCALE), SnapConfig::default());
        assert_eq!(caught.at, [10.0, 0.0]);
        assert_eq!(caught.kind, Some(SnapKind::Node(nodes[1].0)));
    }

    #[test]
    fn the_grid_catches_what_the_piece_does_not() {
        let nodes = square();
        let caught = resolve([4.05, 5.02], &context(&nodes, SCALE), SnapConfig::default());
        assert_eq!(caught.at, [4.0, 5.0]);
        assert_eq!(caught.kind, Some(SnapKind::Grid));
        let free = resolve([4.5, 5.5], &context(&nodes, 30.0), SnapConfig::default());
        assert_eq!(free.at, [4.5, 5.5], "half a centimetre is out of reach");
        assert_eq!(free.kind, None);
    }

    #[test]
    fn a_tract_catches_between_two_nodes() {
        let nodes = square();
        let cfg = SnapConfig {
            grid_cm: 0.0,
            ..SnapConfig::default()
        };
        let caught = resolve([4.5, 0.2], &context(&nodes, SCALE), cfg);
        assert_eq!(caught.at, [4.5, 0.0]);
        assert_eq!(
            caught.kind,
            Some(SnapKind::Edge {
                from: nodes[0].0,
                t: 0.45
            })
        );
    }

    #[test]
    fn the_snap_radius_is_in_screen_points_not_centimetres() {
        let nodes = square();
        let cfg = SnapConfig {
            grid_cm: 0.0,
            ..SnapConfig::default()
        };
        let near = resolve([10.5, 0.0], &context(&nodes, SCALE), cfg);
        assert_eq!(near.kind, Some(SnapKind::Node(nodes[1].0)));
        let zoomed = resolve([10.5, 0.0], &context(&nodes, 100.0), cfg);
        assert_eq!(
            zoomed.at,
            [10.5, 0.0],
            "ten times in, half a centimetre is far"
        );
        assert_eq!(zoomed.kind, None);
    }

    #[test]
    fn ctrl_suppresses_every_candidate() {
        let nodes = square();
        let cfg = SnapConfig {
            on: false,
            ..SnapConfig::default()
        };
        let free = resolve([10.05, 0.05], &context(&nodes, SCALE), cfg);
        assert_eq!(free.at, [10.05, 0.05]);
        assert_eq!(free.kind, None);
    }

    #[test]
    fn shift_constrains_to_the_axis_of_the_anchor() {
        let nodes = square();
        let cfg = SnapConfig {
            axis: true,
            on: false,
            ..SnapConfig::default()
        };
        let ctx = SnapContext {
            anchor: [4.0, 4.0],
            ..context(&nodes, SCALE)
        };
        let flat = resolve([9.0, 4.6], &ctx, cfg);
        assert_eq!(flat.at, [9.0, 4.0]);
        assert_eq!(flat.kind, Some(SnapKind::Axis));
        let upright = resolve([4.4, 9.0], &ctx, cfg);
        assert_eq!(upright.at, [4.0, 9.0]);
        let slanted = resolve([9.0, 9.2], &ctx, cfg);
        assert!(
            (slanted.at[0] - slanted.at[1]).abs() < 1.0e-9,
            "{slanted:?}"
        );
    }

    #[test]
    fn an_axis_still_gives_way_to_a_node() {
        let nodes = square();
        let cfg = SnapConfig {
            axis: true,
            ..SnapConfig::default()
        };
        let ctx = SnapContext {
            anchor: [0.0, 0.0],
            ..context(&nodes, SCALE)
        };
        let caught = resolve([10.2, 0.0], &ctx, cfg);
        assert_eq!(caught.kind, Some(SnapKind::Node(nodes[1].0)));
    }
}
