use super::source::Source;
use super::translate::{Coords, Translator};
use crate::{Block, Error, Id, InternalPath, Piece};

/// A walk finished: a piece's closed contour, or an internal line's open run.
mod finish;
/// The places and the curves a list of path nodes walks through.
mod steps;

pub(crate) use finish::Run;
use steps::steps;

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

/// What a walk makes of two consecutive places that meet on the imported body
/// and nowhere in their formulas.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Coincide {
    /// Refuse the walk. Merging them would cost a contour a tract on the
    /// strength of a coincidence that holds for one body only, and keeping
    /// both would leave a corner no cloth turns at.
    Refuse,
    /// Keep both. A drawn line joins them by a span of no length, which is
    /// what the file wrote and costs a drawing nothing: no cloth is cut along
    /// an internal line, so no claim about the two has to be made at all.
    Keep,
}

/// A piece's outline as a closed run of tracts, in the file's walking order.
pub(crate) fn walk(
    tr: &mut Translator<'_>,
    block: &Block,
    piece: &Piece,
) -> Result<Vec<Tract>, Error> {
    let steps = steps(tr, &piece.outline)?;
    let steps = collapse(tr, block, &piece.name, steps, Coincide::Refuse)?;
    finish::closed(tr, &piece.name, steps)
}

/// An internal path as an open run of places, in the file's walking order.
///
/// The same walk a contour is made of, finished differently: a line has two
/// ends, so the place it stops at is never folded back onto the place it
/// started from. A pocket slit drawn as a loop closes because its author drew
/// it closed, and dropping that last place would leave it open.
///
/// Two places the formulas cannot prove are one are both kept, where a contour
/// refuses them: the file's own node list said to go from the one to the other,
/// and going nowhere at all is the truest reading of that when the two meet.
pub(crate) fn line(
    tr: &mut Translator<'_>,
    block: &Block,
    path: &InternalPath,
) -> Result<Run, Error> {
    let steps = steps(tr, &path.nodes)?;
    let steps = collapse(tr, block, &path.name, steps, Coincide::Keep)?;
    finish::open(&path.name, steps)
}

/// Merges consecutive vertices that are one place — a node and the curve that
/// starts on it — and names the ones an arc produced after the construction
/// point they stand on.
fn collapse(
    tr: &mut Translator<'_>,
    block: &Block,
    walked: &str,
    steps: Vec<Step>,
    coincide: Coincide,
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
            && same(tr, walked, held, next, coincide)?
        {
            if held.point.is_none() {
                *held = next.clone();
            }
            continue;
        }
        out.push(step);
    }
    Ok(out)
}

/// Whether two vertices are one place: the same point, or the same formulas.
///
/// Two vertices that meet on the imported body but not in their formulas meet
/// only as far as the translation can tell, so they are never merged on that
/// strength; what a walk does with the pair instead is its own to say.
fn same(
    tr: &Translator<'_>,
    walked: &str,
    a: &Vertex,
    b: &Vertex,
    coincide: Coincide,
) -> Result<bool, Error> {
    if (a.point.is_some() && a.point == b.point) || a.coords == b.coords {
        return Ok(true);
    }
    let (Some(pa), Some(pb)) = (a.source.locate(tr.reference), b.source.locate(tr.reference))
    else {
        return Ok(false);
    };
    let meet = (pa[0] - pb[0]).abs() <= SAME_PLACE && (pa[1] - pb[1]).abs() <= SAME_PLACE;
    if meet && coincide == Coincide::Refuse {
        return Err(Error::Product(format!(
            "in `{walked}`, two consecutive places meet on the imported body but not by construction"
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
