use super::contact::{Floor, Grip};
use super::hang::Hung;
use super::sdf::SdfGrid;
use super::solver::GRAVITY;

/// Everything a substep asks of the world outside the cloth.
///
/// The body it falls on, the ground it may not fall through, the pull on it,
/// how those two surfaces hold what touches them, and what of the cloth is
/// hung from the body's own rings. They travel together because a substep
/// reads them together, and because the alternative is a parameter each.
/// Gravity is here rather than a constant so that a garment can be sewn shut
/// before it is allowed to fall: [`Stage::weightless`] is that moment, and
/// nothing else changes.
#[derive(Debug, Clone, Copy)]
#[must_use]
pub struct Stage<'a> {
    /// The body the cloth collides with.
    pub sdf: &'a SdfGrid,
    /// The plane under it, when the scene has one.
    pub floor: Floor,
    /// The pull on every particle, in metres per second squared.
    pub gravity: f32,
    /// How the body and the ground hold what is pressed against them.
    pub grip: Grip,
    /// The runs of cloth held at the body's own ring heights; empty for a
    /// product hung from nothing, and then the substep runs exactly the passes
    /// it has always run.
    pub hung: &'a [Hung],
}

impl<'a> Stage<'a> {
    /// A body hanging in the void, under ordinary gravity, holding what
    /// touches it by the fixed share it always has: the scene every drape
    /// golden is taken in.
    pub const fn around(sdf: &'a SdfGrid) -> Stage<'a> {
        Stage {
            sdf,
            floor: Floor::none(),
            gravity: GRAVITY,
            grip: Grip::slipping(),
            hung: &[],
        }
    }

    /// The same, with ground under it.
    pub const fn on(self, floor: Floor) -> Stage<'a> {
        Stage {
            sdf: self.sdf,
            floor,
            gravity: self.gravity,
            grip: self.grip,
            hung: self.hung,
        }
    }

    /// The same, with nothing pulling.
    pub const fn weightless(self) -> Stage<'a> {
        Stage {
            sdf: self.sdf,
            floor: self.floor,
            gravity: 0.0,
            grip: self.grip,
            hung: self.hung,
        }
    }

    /// The same, with contacts that read the push rather than take a fixed
    /// share of the motion.
    pub const fn holding(self, grip: Grip) -> Stage<'a> {
        Stage {
            sdf: self.sdf,
            floor: self.floor,
            gravity: self.gravity,
            grip,
            hung: self.hung,
        }
    }

    /// The same, with runs of its cloth held at the body's ring heights.
    pub const fn hung_from(self, hung: &'a [Hung]) -> Stage<'a> {
        Stage {
            sdf: self.sdf,
            floor: self.floor,
            gravity: self.gravity,
            grip: self.grip,
            hung,
        }
    }
}
