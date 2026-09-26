use eframe::egui::Pos2;
use toile_engine::draft::{Binding, PointKey};

use super::super::precision::Typed;
use super::Gesture;
use crate::bind;

/// The nodes taken in hand, as the gesture holds them.
pub fn holding(drag: Drag) -> Gesture {
    Gesture::Drag(Box::new(drag))
}

/// How a point in hand takes the delta of the gesture.
///
/// `Against` is the whole of the tangent pairing, and it is a delta and not a
/// reflection on purpose: a handle that starts opposite its mate about their
/// node stays opposite it under equal and opposite deltas, while forcing the
/// two collinear would have to overwrite a mate written as a formula with a
/// number the person never typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Follow {
    /// It moves with the pointer.
    Along,
    /// It moves the other way, keeping the tangent through its node straight.
    Against,
}

/// One node in hand, with what it was bound to when the gesture took hold.
#[derive(Debug, Clone, PartialEq)]
pub struct Held {
    /// The node itself.
    pub point: PointKey,
    /// What the drawing calls it, for the question a release may ask.
    pub name: String,
    /// What its two coordinates were bound to when it was grabbed.
    pub origin: [Binding; 2],
    /// Where it resolved to then, in centimetres.
    pub from: [f64; 2],
    /// Which way it takes the gesture's delta.
    pub follow: Follow,
}

/// The nodes on their way somewhere, moving together.
///
/// Each carries what it was bound to when the gesture took hold, because the
/// document is written on every frame of the drag and the answer to the
/// question the release asks has to come from before the first one. The first
/// is the one the pointer took hold of: the whole gesture is measured from it.
#[derive(Debug, Clone, PartialEq)]
pub struct Drag {
    /// The nodes in hand, the one under the pointer first.
    pub nodes: Vec<Held>,
    /// Where the pointer was pressed, on the glass.
    pub grab: Pos2,
    /// Where the node under the pointer has been taken, in centimetres.
    pub to: [f64; 2],
    /// Whether the pointer ever really left: a click is not a drag.
    pub moved: bool,
    /// The resolution the last frame wrote at, in centimetres.
    pub step: f64,
    /// Whether the tangent pairing has been broken for this gesture.
    ///
    /// It latches: once `Alt` has let a handle off its mate, letting the key
    /// go does not put the mate back where symmetry would have kept it. A
    /// tangent that healed itself on release would undo the very asymmetry the
    /// key was held down to make.
    pub free: bool,
    /// The exact value being typed, while the precision box is open.
    pub typed: Option<Typed>,
}

impl Drag {
    /// The node the pointer took hold of; the gesture is measured from it.
    pub fn anchor(&self) -> &Held {
        self.nodes.first().expect("a drag holds at least one node")
    }

    /// Every point in hand, in the order the gesture took them.
    ///
    /// This is what the snap has to leave out of its candidates. A point the
    /// gesture is carrying sits where the last frame left it, so catching one
    /// would make the placement depend on the frame before rather than on the
    /// pointer — and a node dragged with its own handles would be catching
    /// its own tangent.
    pub fn keys(&self) -> Vec<PointKey> {
        self.nodes.iter().map(|held| held.point).collect()
    }

    /// How far the gesture has taken its nodes, in centimetres.
    pub fn delta(&self) -> [f64; 2] {
        let from = self.anchor().from;
        [self.to[0] - from[0], self.to[1] - from[1]]
    }

    /// Every point the gesture is still carrying, with the delta it takes.
    ///
    /// A mate dropped by `Alt` stays in hand — the gesture has to remember it
    /// to keep on ignoring it — but it is carried no further.
    pub fn carried(&self) -> impl Iterator<Item = (&Held, [f64; 2])> {
        let delta = self.delta();
        self.nodes
            .iter()
            .filter(move |held| !(self.free && held.follow == Follow::Against))
            .map(move |held| match held.follow {
                Follow::Along => (held, delta),
                Follow::Against => (held, [-delta[0], -delta[1]]),
            })
    }

    /// Where each point in hand is bound now, rounded to `step` centimetres.
    ///
    /// The nodes take the same delta, so a gesture over a whole corner of the
    /// piece keeps its shape, and a handle's mate takes it reversed.
    /// [`bind::placed`] decides what a delta does to the binding it lands on.
    pub fn placed(&self, step: f64) -> Vec<(PointKey, [Binding; 2])> {
        self.carried()
            .map(|(held, delta)| {
                let to = [0, 1].map(|k| {
                    bind::placed(&held.origin[k], held.from[k] + delta[k], delta[k], step)
                });
                (held.point, to)
            })
            .collect()
    }
}
