use super::super::source::{Curve, End, Source};
use super::super::translate::{Coords, Translator};
use super::{Bend, Step, Vertex};
use crate::{Error, Id, NodeKind, ObjectKind, PathNode};

/// The places and curves a list of path nodes walks through, in order.
///
/// Each node contributes its points and curves, walked backwards where the
/// file says to reverse it. A curve whose handles sit on its ends is the
/// straight line it traces.
pub(super) fn steps(tr: &mut Translator<'_>, nodes: &[PathNode]) -> Result<Vec<Step>, Error> {
    let mut steps = Vec::new();
    for node in nodes {
        let id = node.object;
        match node.kind {
            NodeKind::Point => steps.push(Step::Vertex(vertex(tr, id)?)),
            NodeKind::Spline => {
                let ObjectKind::Spline(spline) = &tr.object(id)?.1.kind else {
                    return Err(Error::Product(format!("object {id} is not a spline")));
                };
                let (start, end) = (vertex(tr, spline.start)?, vertex(tr, spline.end)?);
                let points = tr.spline(id)?;
                curve(
                    &mut steps,
                    [start, end],
                    points,
                    Curve::Spline(id),
                    node.reverse,
                );
            }
            NodeKind::SplinePath => {
                let ObjectKind::SplinePath(through) = &tr.object(id)?.1.kind else {
                    return Err(Error::Product(format!("object {id} is not a spline path")));
                };
                let segments = tr.path(id)?;
                let mut order: Vec<usize> = (0..segments.len()).collect();
                if node.reverse {
                    order.reverse();
                }
                for segment in order {
                    let ends = [
                        vertex(tr, through[segment].point)?,
                        vertex(tr, through[segment + 1].point)?,
                    ];
                    let curve_id = Curve::PathSegment { path: id, segment };
                    curve(
                        &mut steps,
                        ends,
                        segments[segment].clone(),
                        curve_id,
                        node.reverse,
                    );
                }
            }
            NodeKind::Arc => {
                let spans = tr.arc(id)?;
                let count = spans.len();
                let mut order: Vec<usize> = (0..count).collect();
                if node.reverse {
                    order.reverse();
                }
                for index in order {
                    let span = spans[index].clone();
                    let end = |at: usize, coords: Coords| Vertex {
                        coords,
                        point: None,
                        source: Source::ArcPoint {
                            arc: id,
                            index: at,
                            spans: count,
                        },
                    };
                    let ends = [
                        end(index, span.start.clone()),
                        end(index + 1, span.end.clone()),
                    ];
                    let points = [span.start, span.out, span.into, span.end];
                    let curve_id = Curve::ArcSpan {
                        arc: id,
                        index,
                        spans: count,
                    };
                    curve(&mut steps, ends, points, curve_id, node.reverse);
                }
            }
        }
    }
    Ok(steps)
}

fn vertex(tr: &mut Translator<'_>, id: Id) -> Result<Vertex, Error> {
    Ok(Vertex {
        coords: tr.define(id)?,
        point: Some(id),
        source: Source::Point(id),
    })
}

/// Pushes a curve and its two ends, in walking order.
fn curve(
    steps: &mut Vec<Step>,
    [a, b]: [Vertex; 2],
    points: [Coords; 4],
    id: Curve,
    reverse: bool,
) {
    let [start, out, into, end] = points;
    let straight = out == start && into == end;
    let first = (
        out,
        Source::Handle {
            curve: id,
            end: End::First,
        },
    );
    let last = (
        into,
        Source::Handle {
            curve: id,
            end: End::Last,
        },
    );
    let (a, b, first, last) = if reverse {
        (b, a, last, first)
    } else {
        (a, b, first, last)
    };
    steps.push(Step::Vertex(a));
    if !straight {
        steps.push(Step::Bend(Bend {
            out: first,
            into: last,
        }));
    }
    steps.push(Step::Vertex(b));
}
