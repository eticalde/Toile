use toile_engine::draft::{
    Command, Draft, InternalLine, LineKey, LineKind, LineSpan, LineVertex, NotchCount, NotchKey,
    PieceKey, PointKey, curve,
};

use super::gesture::{Feedback, Gesture, Stack};
use super::pick::{self, away, nearest_on};
use super::state::Selection;
use super::tract::{self, Tract};

/// The name a line rubbed out leaves in the undo stack, from the panel's press
/// and from the key alike.
pub const ERASE: &str = "borrar línea";

/// One internal line as the mat draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawn {
    /// The line.
    pub line: LineKey,
    /// What it is for, which is how it is stroked.
    pub kind: LineKind,
    /// The number the panel lists it under, among the lines of its piece.
    pub ordinal: usize,
    /// The name its author gave it, when it carries one.
    pub label: Option<String>,
    /// How many places the line runs through, which is how many times somebody
    /// pressed. Not the length of `run`: a span that bends is flattened there.
    pub places: usize,
    /// The run as a line on the mat; empty when a place of it resolves
    /// nowhere.
    pub run: Vec<[f64; 2]>,
    /// Its places off the contour, where the mat draws them, in run order.
    ///
    /// These are the ones a hand can take hold of and move: each is a point of
    /// the document with two bindings of its own. A place on the contour is not
    /// here — it is a node key and a fraction along the tract leaving it, so
    /// moving it is a question about the contour and not about a point.
    pub loose: Vec<(PointKey, [f64; 2])>,
}

/// One notch as the mat draws it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tick {
    /// The notch.
    pub notch: NotchKey,
    /// How many cuts the mark is.
    pub count: NotchCount,
    /// Where it sits on the contour, in centimetres.
    pub at: [f64; 2],
    /// The unit vector the contour runs in there, so the cut goes across it.
    pub along: [f64; 2],
}

/// Every internal line of a piece, in key order.
///
/// `shift` is how far the view moves the piece from its own coordinates, which
/// the whole product does and a piece on its own does not. It reaches only the
/// free places and the handles: a place on the contour is read off `tracts`,
/// which already lie where the mat draws them.
pub fn of(draft: &Draft, piece: PieceKey, tracts: &[Tract], shift: [f64; 2]) -> Vec<Drawn> {
    draft
        .doc()
        .lines
        .iter()
        .filter(|(_, held)| held.piece == piece)
        .enumerate()
        .map(|(index, (key, held))| Drawn {
            line: key,
            kind: held.kind,
            ordinal: index + 1,
            label: held.label.clone(),
            places: held.spans.len() + 1,
            run: run(draft, tracts, held, shift),
            loose: loose(draft, held, shift),
        })
        .collect()
}

/// The same lines for a piece on its own, flattening its contour to find them.
///
/// What the panel asks, which has the document but not the mat's own work.
pub fn on(draft: &Draft, piece: PieceKey) -> Vec<Drawn> {
    of(draft, piece, &tract::of(draft, piece), [0.0, 0.0])
}

/// Every notch cut into a piece's contour, in key order.
pub fn ticks(draft: &Draft, piece: PieceKey, tracts: &[Tract]) -> Vec<Tick> {
    draft
        .doc()
        .notches
        .iter()
        .filter(|(_, held)| held.at.piece == piece)
        .filter_map(|(key, held)| {
            let tract = tracts.iter().find(|it| it.node == held.at.from)?;
            let (at, along) = along(&tract.line, held.at.t)?;
            Some(Tick {
                notch: key,
                count: held.count,
                at,
                along,
            })
        })
        .collect()
}

/// The internal line within `reach` centimetres of a place on the mat, the
/// nearest when several are; between two as near, the one drawn last.
pub fn under(at: [f64; 2], drawn: &[Drawn], reach: f64) -> Option<LineKey> {
    let mut best: Option<(f64, LineKey)> = None;
    for it in drawn {
        let away = it
            .run
            .windows(2)
            .map(|pair| nearest_on(pair[0], pair[1], at, 0).away)
            .fold(f64::INFINITY, f64::min);
        if away < reach && best.is_none_or(|(kept, _)| away <= kept) {
            best = Some((away, it.line));
        }
    }
    best.map(|(_, line)| line)
}

/// The place of the chosen line a press takes hold of, when one is under it.
///
/// Only the chosen line's places answer: a press that hunted every place of
/// every line would take hold of a mark nobody was looking at. Within a reach
/// and the nearest of them, which is the ladder's first rung — a place is
/// caught the way a node is.
///
/// Only its places off the contour, too, and no edit slides an anchored one. An
/// anchor is a node key and a fraction along the tract leaving it, so sliding
/// it would want a command of its own, with its own inverse and its own folding
/// — and rubbing the line out and tracing it again already moves it, in one
/// entry, with the tool the hand is holding. The day a pattern needs a pocket
/// mouth nudged along its seam without being redrawn, that edit is the one to
/// write.
pub fn grip(at: [f64; 2], drawn: &[Drawn], line: LineKey, reach: f64) -> Option<PointKey> {
    let it = drawn.iter().find(|it| it.line == line)?;
    pick::nearest_node(at, &it.loose, &[], reach).map(|(key, _)| key)
}

/// Where one of these lines puts a place of its own, whichever line holds it.
pub fn at(drawn: &[Drawn], point: PointKey) -> Option<[f64; 2]> {
    drawn
        .iter()
        .flat_map(|it| it.loose.iter().copied())
        .find_map(|(key, place)| (key == point).then_some(place))
}

/// Rubbing one line out, as the one entry a press or a key leaves.
pub fn erased(line: LineKey) -> (Gesture, Vec<Command>, Feedback) {
    (
        Gesture::Idle,
        vec![Command::RemoveLine { line }],
        Feedback {
            stack: Some(Stack::Once(ERASE)),
            select: Some(Selection::None),
            ..Feedback::default()
        },
    )
}

/// The run as a line on the mat, and nothing at all when one place of it
/// resolves nowhere.
///
/// All or nothing on purpose: a run drawn from the places that still resolve
/// would go somewhere the pattern never said, and whoever saw it would believe
/// it. The panel says which line it is instead.
fn run(draft: &Draft, tracts: &[Tract], held: &InternalLine, shift: [f64; 2]) -> Vec<[f64; 2]> {
    let Some(head) = place(draft, tracts, held.head, shift) else {
        return Vec::new();
    };
    let mut out = vec![head];
    for span in &held.spans {
        let from = *out.last().expect("the run opens on its head");
        let Some(to) = place(draft, tracts, span.to, shift) else {
            return Vec::new();
        };
        if span.segment.bends() {
            // Painted as the line it was written as, not as the chord under it:
            // a span bends because somebody pulled its handles, and drawing the
            // shortcut would show a garment nobody drafted.
            let Some((a, b)) = bent(draft, span, shift) else {
                return Vec::new();
            };
            out.extend(
                curve::flatten(from, a, b, to, span.samples)
                    .into_iter()
                    .skip(1),
            );
        }
        out.push(to);
    }
    out
}

/// The line's places off the contour, where the mat draws them.
///
/// A place that resolves nowhere is left out rather than drawn at the origin:
/// the run says so in its own way, by drawing nothing at all.
fn loose(draft: &Draft, held: &InternalLine, shift: [f64; 2]) -> Vec<(PointKey, [f64; 2])> {
    held.vertices()
        .filter_map(LineVertex::point)
        .filter_map(|key| Some((key, moved(draft.resolved(key)?, shift))))
        .collect()
}

/// Where one place of a line falls on the mat, and nothing when it falls
/// nowhere.
fn place(draft: &Draft, tracts: &[Tract], vertex: LineVertex, shift: [f64; 2]) -> Option<[f64; 2]> {
    match vertex {
        LineVertex::Contour(anchor) => {
            let tract = tracts.iter().find(|it| it.node == anchor.from)?;
            Some(along(&tract.line, anchor.t)?.0)
        }
        LineVertex::Free { point } => Some(moved(draft.resolved(point)?, shift)),
    }
}

/// The two handles a bending span hangs on, where the mat draws them.
fn bent(draft: &Draft, span: &LineSpan, shift: [f64; 2]) -> Option<([f64; 2], [f64; 2])> {
    let (out, into) = span.segment.handles()?;
    let at = |key| Some(moved(draft.resolved(key)?, shift));
    Some((at(out)?, at(into)?))
}

fn moved(at: [f64; 2], shift: [f64; 2]) -> [f64; 2] {
    [at[0] + shift[0], at[1] + shift[1]]
}

/// The place a fraction of the way along a line, and the way the line runs
/// there.
///
/// Arc length, which is what a fraction of a tract means everywhere else in the
/// tab. A line of no length at all still answers with its own first place, so a
/// mark on a tract whose two nodes have met is drawn where they met rather than
/// vanishing.
fn along(line: &[[f64; 2]], t: f64) -> Option<([f64; 2], [f64; 2])> {
    let total: f64 = line.windows(2).map(|pair| away(pair[0], pair[1])).sum();
    let wanted = t.clamp(0.0, 1.0) * total;
    let mut walked = 0.0;
    let mut last = None;
    for pair in line.windows(2) {
        let span = away(pair[0], pair[1]);
        if span <= f64::EPSILON {
            continue;
        }
        let unit = [0, 1].map(|k| (pair[1][k] - pair[0][k]) / span);
        if wanted <= walked + span {
            let step = wanted - walked;
            return Some(([0, 1].map(|k| pair[0][k] + unit[k] * step), unit));
        }
        walked += span;
        last = Some((pair[1], unit));
    }
    last.or_else(|| line.first().map(|&at| (at, [1.0, 0.0])))
}

#[cfg(test)]
pub(super) mod tests;
