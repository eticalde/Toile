use super::contact::{Floor, Grip};
use super::sdf::SdfGrid;
use super::solver::GRAVITY;

/// Everything a substep asks of the world outside the cloth: the body it
/// falls on, the ground it may not fall through, the pull on it, and how those
/// two surfaces hold what touches them.
///
/// The four travel together because a substep reads them together, and
/// because the alternative is an eighth parameter. Gravity is here rather
/// than a constant so that a garment can be sewn shut before it is allowed to
/// fall: [`Stage::weightless`] is that moment, and nothing else changes.
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
        }
    }

    /// The same, with ground under it.
    pub const fn on(self, floor: Floor) -> Stage<'a> {
        Stage {
            sdf: self.sdf,
            floor,
            gravity: self.gravity,
            grip: self.grip,
        }
    }

    /// The same, with nothing pulling.
    pub const fn weightless(self) -> Stage<'a> {
        Stage {
            sdf: self.sdf,
            floor: self.floor,
            gravity: 0.0,
            grip: self.grip,
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
        }
    }
}
