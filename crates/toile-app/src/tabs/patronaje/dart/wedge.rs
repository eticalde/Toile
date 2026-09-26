use toile_engine::draft::{FoldDirection, Identity, Point, PointKey, WedgeNode};

use super::super::gesture::EditContext;
use super::LEGS;
use crate::bind;

/// The resolution the wedge's offsets are written at: none of their own.
///
/// Where a loose mark rounds its offset to the snap, a wedge does not. A leg
/// sits on a tract, and an offset pulled to the snap takes it off the very
/// tract it was asked to sit on; the apex's offset is measured from a node the
/// grid does not hold, so rounding it would move the apex off the place the
/// ladder caught it at.
const EXACT: f64 = 0.0;

/// One leg of the wedge, as a press put it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Leg {
    /// The node the tract it sits on leaves.
    pub from: PointKey,
    /// How far along that tract, from zero at its node to one at the next.
    pub t: f64,
    /// Where it lies, in centimetres.
    pub cm: [f64; 2],
}

/// The wedge being cut, between the presses that make a dart.
///
/// Nothing has reached the document yet, which is what lets Escape walk away
/// from it with no entry to unwind and Backspace take the last leg back for
/// free.
#[derive(Debug, Clone, PartialEq)]
pub struct Darting {
    /// The legs pressed so far, in the order they were pressed.
    pub legs: Vec<Leg>,
    /// Where the next press would land, in centimetres.
    pub rubber: [f64; 2],
}

/// The wedge's three nodes as the document will hold them: the bindings of the
/// node the wedge follows, and each node's own offset from it.
///
/// A place written as a pair of plain numbers stops meaning anything the moment
/// the pattern is re-drafted on another body — the cloth moves and the wedge
/// stays behind — so the three are stated from a node instead, which is the
/// rule a loose mark on the cloth is already written by.
///
/// One parent for the three, and it is the node the edit already names rather
/// than the nearest node to each: two legs following two nodes would open and
/// shut the mouth every time those two moved apart, and how much cloth a dart
/// takes out is not the tool's to decide.
///
/// What it costs: the offset is a frozen vector, so the wedge follows the cloth
/// but not the turn of the tract its legs sit on, and a contour that turns as
/// it is re-drafted leaves them a little off the line.
pub(super) fn stated(
    legs: (Leg, Leg),
    apex: [f64; 2],
    ctx: &EditContext<'_>,
) -> Option<[WedgeNode; 3]> {
    let (from, held) = parent(legs.0.from, ctx)?;
    let node = |at: [f64; 2]| {
        WedgeNode::line(
            Identity::New,
            Point::at(
                bind::placed(&held.x, at[0], at[0] - from[0], EXACT),
                bind::placed(&held.y, at[1], at[1] - from[1], EXACT),
            ),
        )
    };
    Some([node(legs.0.cm), node(apex), node(legs.1.cm)])
}

/// Where the node the wedge follows lies, and what the document writes it as.
fn parent<'a>(node: PointKey, ctx: &EditContext<'a>) -> Option<([f64; 2], &'a Point)> {
    let at = ctx.nodes.iter().find(|&&(key, _)| key == node)?.1;
    Some((at, ctx.doc.points.get(node)?))
}

impl Darting {
    /// Which way the dart would be pressed if the apex went down now.
    ///
    /// Toward the leg pressed first. Which way a sewn dart lies is the one
    /// thing about it the geometry cannot say, and the order of the two presses
    /// is information the gesture already has, so it costs no third press to
    /// ask for. The mat draws the arrow from the second press onward, so it is
    /// read before the apex is placed, and Backspace takes the leg back.
    pub fn fold(&self) -> FoldDirection {
        match self.legs.as_slice() {
            [first, second] if second.t < first.t => FoldDirection::TowardEnd,
            _ => FoldDirection::TowardStart,
        }
    }

    /// The two legs in contour order, once both have been pressed.
    pub fn ordered(&self) -> Option<(Leg, Leg)> {
        let [first, second] = <[Leg; LEGS]>::try_from(self.legs.as_slice()).ok()?;
        Some(if first.t <= second.t {
            (first, second)
        } else {
            (second, first)
        })
    }
}
