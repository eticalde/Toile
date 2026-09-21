use crate::eval::angle;
use crate::{CircleArc, Evaluation, Id, Xy};

/// Where a point of the product comes from in the Seamly pattern: what to
/// check it against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// A construction point.
    Point(Id),
    /// A point of an arc cut into `spans` spans of equal sweep: where span
    /// `index` begins, `index == spans` being the arc's end.
    ArcPoint {
        /// The arc.
        arc: Id,
        /// How many spans along.
        index: usize,
        /// How many spans the arc is cut into.
        spans: usize,
    },
    /// One of the two inner control points of a cubic.
    Handle {
        /// The cubic.
        curve: Curve,
        /// Which of the two.
        end: End,
    },
}

/// A cubic of the pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Curve {
    /// A spline.
    Spline(Id),
    /// One segment of a spline path, counted from its first point.
    PathSegment {
        /// The path.
        path: Id,
        /// Which segment.
        segment: usize,
    },
    /// One span of an arc cut into `spans` spans of equal sweep.
    ArcSpan {
        /// The arc.
        arc: Id,
        /// Which span.
        index: usize,
        /// How many spans the arc is cut into.
        spans: usize,
    },
}

/// Which of a cubic's two handles, in the cubic's own direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// The handle leaving its first point.
    First,
    /// The handle arriving at its last point.
    Last,
}

impl Source {
    /// Where `evaluation` puts this point.
    ///
    /// For a construction point or a spline's handle that is the evaluator's
    /// own answer. For an arc, the evaluator gives the circle, and the spans
    /// and their handles are worked out from it here, in floating point, the
    /// way the product works them out in formulas.
    pub fn locate(&self, evaluation: &Evaluation) -> Option<Xy> {
        match *self {
            Source::Point(id) => evaluation.points.get(&id).map(|point| point.at),
            Source::ArcPoint { arc, index, spans } => {
                Some(on_arc(evaluation.arcs.get(&arc)?, index, spans).0)
            }
            Source::Handle { curve, end } => {
                let [_, first, last, _] = controls(evaluation, curve)?;
                Some(match end {
                    End::First => first,
                    End::Last => last,
                })
            }
        }
    }
}

/// A cubic's four control points, as `evaluation` puts them.
fn controls(evaluation: &Evaluation, curve: Curve) -> Option<[Xy; 4]> {
    match curve {
        Curve::Spline(id) => evaluation.splines.get(&id).map(|cubic| cubic.0),
        Curve::PathSegment { path, segment } => evaluation
            .paths
            .get(&path)?
            .get(segment)
            .map(|cubic| cubic.0),
        Curve::ArcSpan { arc, index, spans } => {
            let arc = evaluation.arcs.get(&arc)?;
            let (start, [lx, ly]) = on_arc(arc, index, spans);
            let (end, [ax, ay]) = on_arc(arc, index + 1, spans);
            let reach = arc.radius * angle::arc_handle_ratio(arc.sweep() / spans as f64);
            Some([
                start,
                [start[0] + reach * lx, start[1] + reach * ly],
                [end[0] - reach * ax, end[1] - reach * ay],
                end,
            ])
        }
    }
}

/// The point `index` spans along an arc cut into `spans`, and the unit
/// vector along which the arc leaves it counter-clockwise.
fn on_arc(arc: &CircleArc, index: usize, spans: usize) -> (Xy, Xy) {
    let degrees = arc.angle1 + arc.sweep() * index as f64 / spans as f64;
    let [x, y] = angle::heading(degrees);
    let point = [
        arc.center[0] + arc.radius * x,
        arc.center[1] + arc.radius * y,
    ];
    (point, [y, -x])
}
