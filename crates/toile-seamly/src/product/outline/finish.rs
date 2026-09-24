use super::super::translate::Translator;
use super::{Bend, Coincide, Step, Tract, Vertex, same};
use crate::Error;

/// An open run of places: where it starts, and every further place with what
/// reaches it.
///
/// A head and its reaches rather than two lists side by side, which is how the
/// document holds an internal line too: with one reach per further place, a
/// curve that runs to nowhere cannot be written down.
#[derive(Debug, Clone)]
pub(crate) struct Run {
    pub(crate) head: Vertex,
    pub(crate) spans: Vec<Reach>,
}

/// One step of an open run: what runs, and the place it reaches.
#[derive(Debug, Clone)]
pub(crate) struct Reach {
    /// The curve that reaches the place; `None` for a straight line.
    pub(crate) bend: Option<Bend>,
    pub(crate) to: Vertex,
}

/// The walk as the closed contour of a piece.
///
/// The last place is folded onto the first when the two are one, since a
/// contour closes on itself and a doubled corner is a tract of no length.
pub(crate) fn closed(
    tr: &Translator<'_>,
    piece: &str,
    mut steps: Vec<Step>,
) -> Result<Vec<Tract>, Error> {
    if let (Some(Step::Vertex(first)), Some(Step::Vertex(last))) = (steps.first(), steps.last())
        && steps.len() > 1
        && same(tr, piece, first, last, Coincide::Refuse)?
    {
        steps.pop();
    }
    let mut out = Vec::new();
    for (from, bend) in paired(piece, steps)? {
        out.push(Tract { from, bend });
    }
    if out.len() < 3 {
        return Err(Error::Product(format!(
            "the outline of `{piece}` has fewer than three corners"
        )));
    }
    Ok(out)
}

/// The walk as the open run of an internal line.
///
/// A run of one place is handed back as it is rather than refused: nothing can
/// be drawn from it, and which path it was is what the import has to say.
pub(crate) fn open(walked: &str, steps: Vec<Step>) -> Result<Run, Error> {
    let mut places = paired(walked, steps)?.into_iter();
    let (head, mut carried) = places
        .next()
        .ok_or_else(|| Error::Product(format!("`{walked}` walks nowhere at all")))?;
    let mut spans = Vec::new();
    for (to, bend) in places {
        spans.push(Reach { bend: carried, to });
        carried = bend;
    }
    if carried.is_some() {
        return Err(Error::Product(format!(
            "`{walked}` ends on a curve that reaches no place"
        )));
    }
    Ok(Run { head, spans })
}

/// Every place of the walk with the curve that leaves it, if one does.
fn paired(walked: &str, steps: Vec<Step>) -> Result<Vec<(Vertex, Option<Bend>)>, Error> {
    let broken = || {
        Error::Product(format!(
            "the walk of `{walked}` does not alternate places and curves"
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
        out.push((from, bend));
    }
    Ok(out)
}
