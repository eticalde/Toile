use toile_engine::draft::PointKey;

use super::pick::{self, EDGE_PT, NODE_PT};
use super::tract::{self, Tract};

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
/// The ladder is fixed and it is short: node, then handle, then tract, then
/// the axis of the gesture, then the drawn grid. Crossings join it on the day
/// the tool that draws them does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SnapKind {
    /// An existing node of the piece.
    Node(PointKey),
    /// A handle of one of its tracts, out of the ones on show.
    Handle(PointKey),
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
    /// The handles on show, resolved, in contour order.
    pub handles: &'a [(PointKey, [f64; 2])],
    /// Its tracts as the drawing paints them, curves flattened.
    pub tracts: &'a [Tract],
    /// The points in hand, which catch neither themselves nor their own
    /// tracts.
    ///
    /// Every one of them, not only the one the pointer took hold of: a
    /// gesture carries the whole selection and the handles hanging off it,
    /// and those sit where the last frame left them. Catching one would make
    /// the placement answer to the frame before instead of to the pointer.
    pub held: &'a [PointKey],
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
    if let Some((key, place)) = pick::nearest_node(at, ctx.handles, ctx.held, reach(NODE_PT)) {
        return Snapped {
            at: place,
            kind: Some(SnapKind::Handle(key)),
        };
    }
    if let Some(found) = tract::nearest(at, ctx.tracts, ctx.held)
        && found.away < reach(EDGE_PT)
    {
        return Snapped {
            at: found.at,
            kind: Some(SnapKind::Edge {
                from: ctx.tracts[found.from].node,
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
mod tests;
