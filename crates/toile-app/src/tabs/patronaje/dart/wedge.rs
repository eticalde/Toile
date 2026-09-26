use toile_engine::draft::{FoldDirection, PointKey};

use super::LEGS;

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
