use std::sync::{Arc, OnceLock};

use toile_anny::BodyMesh;
use toile_sim::xpbd::{Grip, SdfGrid};

use super::bake::{self, BakeError};
use super::belt::{self, Belt};
use crate::{couture, demo};

/// How far above a body a garment is let go, in metres.
///
/// The demo scene's own gap, read back off it: its sphere reaches 0.15 m and
/// the bodice is released at 0.35. Deriving every other body's release from
/// that same gap is what puts one rule behind both, instead of a literal for
/// the sphere and a guess for a person.
pub const CLEARANCE: f32 = 0.20;

/// How a body's skin holds cloth: the static coefficient, then the kinetic
/// one.
///
/// Textile against dry human skin is measured between about 0.3 and 0.7 —
/// Derler and Gerhardt's 2012 review of skin friction collects the range, and
/// cotton on a dry forearm sits near its middle; wet skin goes past one. These
/// two sit in the upper half of the dry range and keep the usual ratio of
/// about two thirds between sliding and holding, which is what gives a
/// waistband something to catch on before it moves.
///
/// Every baked body gets this grip and only the demo sphere does not: no
/// document field, setting or control carries it, so a person can neither see
/// it nor turn it off. That is a product decision made here by a constant.
const SKIN: (f32, f32) = (0.6, 0.4);

/// The body a drape falls on: the field the solver collides against, where a
/// garment is let go over it, and the rings that say where one belongs on it.
///
/// The field's own box answers none of the three. The demo sphere bakes a
/// 1.4 m cube around a ball 30 cm across, so a height taken off the grid
/// would release the bodice most of a metre above a body that is not there.
/// What travels here is the body's own extent, and its own measurements.
///
/// Cloning is cheap and shares the voxels: the field is written once by the
/// bake and read for the rest of its life.
#[derive(Clone)]
pub struct Collider {
    field: Arc<SdfGrid>,
    release: f32,
    lo: [f32; 3],
    hi: [f32; 3],
    ground: Option<f32>,
    grip: Grip,
    belts: Arc<[Belt]>,
}

impl Collider {
    /// The eternal fixture: the demo sphere, released from the height the
    /// goldens were taken at.
    ///
    /// The release is [`couture::DROP_HEIGHT`] itself rather than the sphere's
    /// top plus [`CLEARANCE`]. The two agree to the centimetre and not to the
    /// bit, and this one is the number the drape golden stands on.
    /// It stands on nothing, and that is the point: a ball hanging in the
    /// void is the scene every drape golden is taken in, and a floor under it
    /// would be a different scene.
    pub fn demo() -> Collider {
        let r = demo::AVATAR_RADIUS;
        Collider {
            field: demo_field(),
            release: couture::DROP_HEIGHT,
            lo: [-r; 3],
            hi: [r; 3],
            ground: None,
            // The sphere holds cloth the way it always has. It is the physics
            // reference and every drape golden is hashed off it, so the one
            // body in the tree that must never read the push is this one.
            grip: Grip::slipping(),
            belts: Arc::from([]),
        }
    }

    /// Bakes a body mesh into the field a garment will fall on.
    ///
    /// Half a second and tens of megabytes for an adult, so this belongs off
    /// whatever thread is drawing — see [`super::oven`].
    ///
    /// # Errors
    /// `BakeError` when the mesh is not the closed, orientable surface the
    /// sign needs; see [`bake::sdf`].
    pub fn bake(mesh: &BodyMesh) -> Result<Collider, BakeError> {
        Ok(Collider::over(bake::sdf(mesh)?, mesh))
    }

    /// A field already baked, put back over the body it was baked from.
    ///
    /// The cache stores the voxels and not the body, so the extent and the
    /// rings are measured from the mesh again rather than written down twice
    /// and trusted.
    pub(crate) fn over(field: SdfGrid, mesh: &BodyMesh) -> Collider {
        let (lo, hi) = extent(&mesh.positions);
        Collider {
            field: Arc::new(field),
            release: hi[1] + CLEARANCE,
            lo,
            hi,
            ground: Some(lo[1]),
            grip: Grip::coulomb(SKIN.0, SKIN.1),
            belts: Arc::from(belt::of(mesh)),
        }
    }

    /// Where a garment is let go over this body, in metres.
    pub fn release_height(&self) -> f32 {
        self.release
    }

    /// The body's lowest and highest corner, in metres.
    pub fn extent(&self) -> ([f32; 3], [f32; 3]) {
        (self.lo, self.hi)
    }

    /// The plane this body stands on, in metres; `None` for one that stands
    /// on nothing.
    ///
    /// A body's own lowest point, because that is what standing means: the
    /// Anny body is modelled soles-down and reaches its lowest at the feet —
    /// the reference adult at −0.837 m, nowhere near the zero a floor would
    /// be at if it were guessed. Taking it off the mesh rather than writing a
    /// number down is also what lets a shorter body stand on its own floor
    /// instead of hovering over somebody else's.
    ///
    /// Plain metres rather than the solver's own type: the interface draws
    /// this ground, and `toile-app` never sees `toile-sim`.
    pub fn ground(&self) -> Option<f32> {
        self.ground
    }

    /// How this body's surface holds cloth pressed against it.
    ///
    /// A baked person's skin reads the push and can hold a garment up; the
    /// demo sphere takes the fixed share of the motion it always has. That is
    /// the whole of who asks for the new contact and who does not.
    pub(crate) fn grip(&self) -> Grip {
        self.grip
    }

    /// The body's own measurement rings; empty for one they were not cut for.
    pub(crate) fn belts(&self) -> &[Belt] {
        &self.belts
    }

    /// Whether a point is under this body's skin.
    ///
    /// The field's own sign, read exactly where and how the solver reads it,
    /// so that "inside the body" cannot come to mean one thing to a caller
    /// asking and another to the drape being asked about.
    pub fn contains(&self, p: [f32; 3]) -> bool {
        self.field.sample(p[0], p[1], p[2]) < 0.0
    }

    /// Whether a point is so far under the skin that nothing will carry it
    /// out again.
    ///
    /// Past the band the field is saturated flat: its gradient is exactly
    /// zero, so the contact solve has no normal to push along and the
    /// particle stays wherever it was put. Cloth resting on a body straddles
    /// the surface by a fraction of a millimetre and is not this; a garment
    /// let go with cloth in here never recovers.
    pub fn swallows(&self, p: [f32; 3]) -> bool {
        self.field.sample(p[0], p[1], p[2]) <= -(bake::BAND as f32)
    }

    /// Samples along x, y and z.
    pub fn dims(&self) -> [usize; 3] {
        self.field.dims
    }

    /// The field itself, for a caller inside the engine that collides against
    /// it directly rather than through the sim thread.
    pub(crate) fn field(&self) -> &SdfGrid {
        &self.field
    }

    /// The field as the sim thread takes it: a second owner of the same
    /// voxels, so handing a body to the solver copies a pointer and not forty
    /// megabytes.
    pub(crate) fn shared(&self) -> Arc<SdfGrid> {
        self.field.clone()
    }
}

/// The demo sphere's field, baked once per process and shared from there.
///
/// Every session that has not been handed a body falls back to it, including
/// a blank table that never drapes anything, and 16.7 million samples of a
/// field that cannot change is not a cost worth paying per session.
/// [`demo::avatar_sdf`] itself is untouched: the goldens go on building their
/// own through it, exactly as they always have.
fn demo_field() -> Arc<SdfGrid> {
    static ONCE: OnceLock<Arc<SdfGrid>> = OnceLock::new();
    ONCE.get_or_init(|| Arc::new(demo::avatar_sdf())).clone()
}

/// The lowest and highest corner of a run of xyz positions.
fn extent(positions: &[f32]) -> ([f32; 3], [f32; 3]) {
    let mut lo = [f32::INFINITY; 3];
    let mut hi = [f32::NEG_INFINITY; 3];
    for p in positions.as_chunks::<3>().0 {
        for c in 0..3 {
            lo[c] = lo[c].min(p[c]);
            hi[c] = hi[c].max(p[c]);
        }
    }
    (lo, hi)
}
