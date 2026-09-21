use super::source::{Curve, End, Source};
use super::translate::{Coords, Translator};
use crate::{Block, Error, Id, NodeKind, ObjectKind, Piece};

/// How close, in centimetres, two points of the imported body must be to be
/// taken for the same place when their formulas are compared.
const SAME_PLACE: f64 = 1e-9;

/// A corner of a contour.
#[derive(Debug, Clone)]
pub(crate) struct Vertex {
    /// Its coordinates written out in full, which is how two vertices are
    /// told apart.
    pub(crate) coords: Coords,
    /// The construction point it is, when it is one.
    pub(crate) point: Option<Id>,
    pub(crate) source: Source,
}

/// A curve from one vertex to the next: its two handles.
#[derive(Debug, Clone)]
pub(crate) struct Bend {
    /// The handle leaving the vertex, and where it comes from.
    pub(crate) out: (Coords, Source),
    /// The handle arriving at the next vertex.
    pub(crate) into: (Coords, Source),
}

/// A vertex and what runs from it to the next: a curve, or a straight line.
#[derive(Debug, Clone)]
pub(crate) struct Tract {
    pub(crate) from: Vertex,
    pub(crate) bend: Option<Bend>,
}

#[allow(
    clippy::large_enum_variant,
    reason = "an outline is a few dozen steps, walked once at import"
)]
enum Step {
    Vertex(Vertex),
    Bend(Bend),
}

/// A piece's outline as a closed run of tracts, in the file's walking order.
///
/// Each node contributes its points and curves, walked backwards where the
/// file says to reverse it. A point the walk reaches twice in a row — a node
/// and the curve that starts on it — is one vertex, and so is the last one if
/// it closes on the first. A curve whose handles sit on its ends is the
/// straight line it traces.
pub(crate) fn walk(
    tr: &mut Translator<'_>,
    block: &Block,
    piece: &Piece,
) -> Result<Vec<Tract>, Error> {
    let mut steps = Vec::new();
    for node in &piece.outline {
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
    let steps = collapse(tr, block, &piece.name, steps)?;
    tracts(&piece.name, steps)
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

/// Merges consecutive vertices that are one place, and names the ones an
/// arc produced after the construction point they stand on.
fn collapse(
    tr: &mut Translator<'_>,
    block: &Block,
    piece: &str,
    steps: Vec<Step>,
) -> Result<Vec<Step>, Error> {
    let mut out: Vec<Step> = Vec::with_capacity(steps.len());
    for step in steps {
        let step = match step {
            Step::Vertex(unnamed) if unnamed.point.is_none() => {
                Step::Vertex(named(tr, block, unnamed))
            }
            other => other,
        };
        if let (Some(Step::Vertex(held)), Step::Vertex(next)) = (out.last_mut(), &step)
            && same(tr, piece, held, next)?
        {
            if held.point.is_none() {
                *held = next.clone();
            }
            continue;
        }
        out.push(step);
    }
    if let (Some(Step::Vertex(first)), Some(Step::Vertex(last))) = (out.first(), out.last())
        && out.len() > 1
        && same(tr, piece, first, last)?
    {
        out.pop();
    }
    Ok(out)
}

/// Whether two vertices are one place: the same point, or the same formulas.
///
/// Two vertices that meet on the imported body but not in their formulas
/// would meet only on that body, so they are refused rather than merged.
fn same(tr: &Translator<'_>, piece: &str, a: &Vertex, b: &Vertex) -> Result<bool, Error> {
    if (a.point.is_some() && a.point == b.point) || a.coords == b.coords {
        return Ok(true);
    }
    let (Some(pa), Some(pb)) = (a.source.locate(tr.reference), b.source.locate(tr.reference))
    else {
        return Ok(false);
    };
    if (pa[0] - pb[0]).abs() <= SAME_PLACE && (pa[1] - pb[1]).abs() <= SAME_PLACE {
        return Err(Error::Product(format!(
            "in `{piece}`, two consecutive corners meet on the imported body but not by construction"
        )));
    }
    Ok(false)
}

/// The construction point of `block` an arc's vertex stands on, if one has
/// the same formulas; the vertex as it was otherwise.
fn named(tr: &mut Translator<'_>, block: &Block, vertex: Vertex) -> Vertex {
    let Some(at) = vertex.source.locate(tr.reference) else {
        return vertex;
    };
    for object in &block.objects {
        let Some(placed) = tr.reference.points.get(&object.id) else {
            continue;
        };
        let near = (placed.at[0] - at[0]).abs() <= SAME_PLACE
            && (placed.at[1] - at[1]).abs() <= SAME_PLACE;
        if near && tr.define(object.id).ok().as_ref() == Some(&vertex.coords) {
            return Vertex {
                coords: vertex.coords,
                point: Some(object.id),
                source: Source::Point(object.id),
            };
        }
    }
    vertex
}

fn tracts(piece: &str, steps: Vec<Step>) -> Result<Vec<Tract>, Error> {
    let broken = || {
        Error::Product(format!(
            "the outline of `{piece}` does not alternate corners and curves"
        ))
    };
    let mut out = Vec::new();
    let mut steps = steps.into_iter().peekable();
    while let Some(step) = steps.next() {
        let Step::Vertex(from) = step else {
            return Err(broken());
        };
        let bend = match steps.next_if(|next| matches!(next, Step::Bend(_))) {
            Some(Step::Bend(bend)) => Some(bend),
            _ => None,
        };
        out.push(Tract { from, bend });
    }
    if out.len() < 3 {
        return Err(Error::Product(format!(
            "the outline of `{piece}` has fewer than three corners"
        )));
    }
    Ok(out)
}
