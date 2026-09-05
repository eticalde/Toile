use toile_engine::draft::{Command, Doc, Draft, PieceKey, Point, PointKey, Segment, SegmentEdit};

use super::state::Selection;

/// How finely a tract the Curve tool draws is flattened, to start with.
///
/// Sixteen holds the polyline within a tenth of a millimetre of the cubic on a
/// tract the size of a crotch curve, which is the resolution a drag and the
/// precision box already round to. The inspector moves it from there.
pub const SAMPLES: u16 = 16;

/// The narrowest and the widest a tract may be flattened to.
///
/// The range is the document's, because a file is written by hand as often as
/// by this program and a panel refuses nothing a file has to pass. What the
/// panel adds is the moment: it says no while the number is being typed,
/// before the paper on the mat is clipped from the polyline it would ask for.
pub const SAMPLE_RANGE: (u16, u16) = toile_engine::draft::SAMPLES;

/// Which end of a tract a handle belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// It leaves the node the tract starts at.
    Out,
    /// It enters the node the tract ends at.
    Into,
}

/// A handle, said as the node it hangs from and the side it lies on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hangs {
    /// The node it pulls the tangent of.
    pub node: PointKey,
    /// The side of the tract it sits on.
    pub side: Side,
    /// The other handle at that node, when the node has one.
    pub mate: Option<PointKey>,
}

/// One bent tract with everything about it resolved.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bend {
    /// The node the tract leaves.
    pub node: PointKey,
    /// Where that node sits, in centimetres.
    pub from: [f64; 2],
    /// The handle leaving it, and where it sits.
    pub out: (PointKey, [f64; 2]),
    /// The handle entering the next node, and where it sits.
    pub into: (PointKey, [f64; 2]),
    /// The node the tract runs to.
    pub to_node: PointKey,
    /// Where that node sits.
    pub to: [f64; 2],
}

/// Every bent tract of a piece, in contour order.
pub fn bends(draft: &Draft, piece: PieceKey) -> Vec<Bend> {
    let doc = draft.doc();
    let Some(held) = doc.pieces.get(piece) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (index, node) in held.contour.iter().enumerate() {
        let Segment::Cubic { out: a, into: b } = node.segment else {
            continue;
        };
        let next = held.contour[(index + 1) % held.contour.len()].point;
        let (Some(from), Some(at_a), Some(at_b), Some(to)) = (
            draft.resolved(node.point),
            draft.resolved(a),
            draft.resolved(b),
            draft.resolved(next),
        ) else {
            continue;
        };
        out.push(Bend {
            node: node.point,
            from,
            out: (a, at_a),
            into: (b, at_b),
            to_node: next,
            to,
        });
    }
    out
}

/// Whether the drawing is showing this bend's handles.
///
/// A bend shows them when either of its nodes is chosen, when the tract itself
/// is, or when one of its own handles is — the last so that grabbing a handle
/// does not make it vanish under the pointer. Every handle at once would bury
/// the piece in leader lines, which is the state of every pattern tool that
/// draws them unasked.
pub fn shown(bend: &Bend, chosen: &Selection) -> bool {
    chosen.edge() == Some(bend.node)
        || chosen.holds(bend.node)
        || chosen.holds(bend.to_node)
        || chosen.holds(bend.out.0)
        || chosen.holds(bend.into.0)
}

/// The handles the drawing is showing, which are the ones a press may grab.
pub fn handles(bends: &[Bend], chosen: &Selection) -> Vec<(PointKey, [f64; 2])> {
    bends
        .iter()
        .filter(|bend| shown(bend, chosen))
        .flat_map(|bend| [bend.out, bend.into])
        .collect()
}

/// Where one of these bends puts a handle, whether it is on show or not.
pub fn at(bends: &[Bend], handle: PointKey) -> Option<[f64; 2]> {
    bends.iter().find_map(|bend| {
        [bend.out, bend.into]
            .into_iter()
            .find_map(|(key, place)| (key == handle).then_some(place))
    })
}

/// The node a handle hangs from, the side it lies on, and its mate there.
pub fn hangs(doc: &Doc, piece: PieceKey, handle: PointKey) -> Option<Hangs> {
    let held = doc.pieces.get(piece)?;
    let count = held.contour.len();
    let index = held
        .contour
        .iter()
        .position(|node| node.segment.cites(handle))?;
    let out = held.contour[index].segment.handles()?.0 == handle;
    let (node, side, across) = if out {
        (
            held.contour[index].point,
            Side::Out,
            (index + count - 1) % count,
        )
    } else {
        (
            held.contour[(index + 1) % count].point,
            Side::Into,
            (index + 1) % count,
        )
    };
    let mate = held.contour[across]
        .segment
        .handles()
        .map(|pair| if out { pair.1 } else { pair.0 })
        .filter(|_| across != index);
    Some(Hangs { node, side, mate })
}

/// The handles that hang from a node: at most one on each side of it.
pub fn hanging(doc: &Doc, piece: PieceKey, node: PointKey) -> Vec<PointKey> {
    let Some(held) = doc.pieces.get(piece) else {
        return Vec::new();
    };
    let count = held.contour.len();
    let Some(index) = held.node_index(node) else {
        return Vec::new();
    };
    let leaving = held.contour[index].segment.handles().map(|pair| pair.0);
    let arriving = held.contour[(index + count - 1) % count]
        .segment
        .handles()
        .map(|pair| pair.1);
    leaving.into_iter().chain(arriving).collect()
}

/// The edit that bends the straight tract leaving `node`, and the one that
/// says how finely to flatten it.
///
/// The handles land on the thirds of the chord, where a cubic is exactly the
/// straight line it replaces: the click gives the tract its handles and not a
/// bulge nobody asked for. They are written as plain numbers, because a
/// tangent the tool invented has no formula to inherit; the inspector and the
/// precision box are where one gets written.
///
/// The count goes first. A tract sampled at one point is flattened as its own
/// chord, so the document refuses to hang handles on it until it is sampled
/// for a curve, and taking the pair back in reverse puts the count back on a
/// tract that has already straightened.
pub fn bend(
    doc: &Doc,
    piece: PieceKey,
    node: PointKey,
    ends: ([f64; 2], [f64; 2]),
) -> Vec<Command> {
    if doc.pieces.get(piece).is_none() {
        return Vec::new();
    }
    let (a, b) = ends;
    let third = |k: f64| [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k];
    let (out, into) = (third(1.0 / 3.0), third(2.0 / 3.0));
    vec![
        Command::SetSamples {
            piece,
            node,
            to: SAMPLES,
        },
        Command::SetSegment {
            piece,
            node,
            to: SegmentEdit::cubic(Point::at(out[0], out[1]), Point::at(into[0], into[1])),
        },
    ]
}

/// How finely the tract leaving `node` is flattened, when it bends at all.
pub fn samples_of(doc: &Doc, piece: PieceKey, node: PointKey) -> Option<u16> {
    let held = doc.pieces.get(piece)?;
    let at = held.node_index(node)?;
    let held = held.contour.get(at)?;
    matches!(held.segment, Segment::Cubic { .. }).then_some(held.samples)
}

#[cfg(test)]
mod tests;
