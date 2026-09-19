use super::sdf::SdfGrid;
use super::state::State;

#[cfg(test)]
mod tests;

/// Fraction of the substep's tangential motion removed on contact. Without
/// friction the garment slides down the field forever and never settles.
const FRICTION: f32 = 0.5;

/// Contact damping, applied by moving `q` toward `p`. PBD velocity is
/// `(p − q)/dt`, so this bleeds off the energy that normal jitter from the
/// trilinear field pumps in, without touching cloth in free flight.
const CONTACT_DAMP: f32 = 0.5;

/// How a surface holds what is pressed against it.
///
/// [`Grip::slipping`] is the arithmetic every scene has run since there was a
/// contact at all: [`FRICTION`] of the substep's tangential motion goes,
/// whatever the surface was pressed with. Nothing that squeezes harder is held
/// better under it, so a contact never sticks and a slide runs at a speed the
/// push cannot change.
///
/// [`Grip::coulomb`] reads the push. The normal correction of this substep is
/// how deep the surface had to move the particle, and the friction is bounded
/// by it: inside `stick × depth` the whole of the sideways motion goes and the
/// contact holds, past that `slide × depth` goes and the rest is a slide. That
/// is the position-level Coulomb projection of Macklin et al., "Small Steps in
/// Physics Simulation".
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Grip(Option<Coulomb>);

/// The two coefficients of a contact that can stick.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Coulomb {
    stick: f32,
    slide: f32,
}

impl Grip {
    /// The fixed share of the motion, taken whatever the push was: the
    /// arithmetic every golden is hashed from.
    pub const fn slipping() -> Grip {
        Grip(None)
    }

    /// Coulomb's two coefficients, static then kinetic.
    pub const fn coulomb(stick: f32, slide: f32) -> Grip {
        Grip(Some(Coulomb { stick, slide }))
    }

    /// The share of this substep's tangential motion the contact takes, from
    /// how deep the normal correction was and how far the particle went
    /// sideways — the latter squared, so a slipping contact never pays for a
    /// root it does not read.
    #[inline]
    fn takes(self, depth: f32, sideways2: f32) -> f32 {
        let Some(mu) = self.0 else {
            return FRICTION;
        };
        let sideways = sideways2.sqrt();
        if sideways <= mu.stick * depth {
            return 1.0;
        }
        (mu.slide * depth / sideways).min(1.0)
    }
}

/// Projects one particle out of the field and applies contact friction and
/// damping. A particle in free flight is returned untouched.
///
/// Takes and returns both the current position `p` and the pre-substep
/// position `q`, because damping contact means moving `q`, not `p`.
#[inline]
pub(super) fn resolve(
    sdf: &SdfGrid,
    eps: f32,
    grip: Grip,
    p: [f32; 3],
    q: [f32; 3],
) -> ([f32; 3], [f32; 3]) {
    let d = sdf.sample(p[0], p[1], p[2]);
    if d >= 0.0 {
        return (p, q);
    }
    let gx = sdf.sample(p[0] + eps, p[1], p[2]) - d;
    let gy = sdf.sample(p[0], p[1] + eps, p[2]) - d;
    let gz = sdf.sample(p[0], p[1], p[2] + eps) - d;
    // Deeper in than the band reaches, the field is saturated flat and there
    // is no normal to be had. The push along it moves the particle nowhere,
    // while the friction below would take away every bit of tangential motion
    // against that same nothing and the damping would stop the particle dead
    // where it stands. [`lift_out_of`] leaves such a particle for the stretch
    // constraints to draw out through its neighbours, and so does this.
    let square = gx * gx + gy * gy + gz * gz;
    if square <= 0.0 {
        return (p, q);
    }
    let glen = square.sqrt().max(1.0e-9);
    let push = -d / glen;
    let mut p = [p[0] + gx * push, p[1] + gy * push, p[2] + gz * push];

    let (nx, ny, nz) = (gx / glen, gy / glen, gz / glen);
    let (mx, my, mz) = (p[0] - q[0], p[1] - q[1], p[2] - q[2]);
    let dn = mx * nx + my * ny + mz * nz;
    let (tx, ty, tz) = (mx - dn * nx, my - dn * ny, mz - dn * nz);
    // `-d` is the depth the push above just covered: the correction ran along
    // the unit normal by exactly that much, which is what Coulomb's bound is
    // taken against.
    let take = grip.takes(-d, tx * tx + ty * ty + tz * tz);
    p[0] -= take * tx;
    p[1] -= take * ty;
    p[2] -= take * tz;

    let q = [
        q[0] + CONTACT_DAMP * (p[0] - q[0]),
        q[1] + CONTACT_DAMP * (p[1] - q[1]),
        q[2] + CONTACT_DAMP * (p[2] - q[2]),
    ];
    (p, q)
}

/// The ground a body stands on: the plane cloth may not fall through, or no
/// ground at all.
///
/// The demo sphere floats, and it is the physics reference, so it carries
/// [`Floor::none`] and every drape golden is taken with nothing underneath.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Floor(Option<f32>);

impl Floor {
    /// No ground: cloth falls for as long as anything integrates it.
    pub const fn none() -> Floor {
        Floor(None)
    }

    /// Ground at `y` metres.
    pub const fn at(y: f32) -> Floor {
        Floor(Some(y))
    }

    /// The plane's height, when there is one.
    pub const fn level(self) -> Option<f32> {
        self.0
    }
}

/// Rests one particle on the plane at `y`, with the grip and the damping the
/// field's own contact uses. A particle above it is returned untouched.
///
/// The plane's normal is `+y` exactly, so what [`resolve`] spends four samples
/// establishing is a constant here: no gradient, no `eps`, and the tangential
/// motion friction takes is simply the horizontal part of the step. There is
/// no saturated interior to guard against either — a plane has a normal
/// everywhere, so a particle under it always has somewhere to be pushed.
///
/// The same `grip` as the body, and not a rule of its own: a hem heaped on the
/// ground is cloth held by a surface it is pressed onto, which is the thing
/// [`Grip`] describes. A scene that wants the two to differ wants two stages.
#[inline]
pub(super) fn rest_on(y: f32, grip: Grip, p: [f32; 3], q: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    if p[1] >= y {
        return (p, q);
    }
    let depth = y - p[1];
    let mut p = [p[0], y, p[2]];
    let (tx, tz) = (p[0] - q[0], p[2] - q[2]);
    let take = grip.takes(depth, tx * tx + tz * tz);
    p[0] -= take * tx;
    p[2] -= take * tz;
    let q = [
        q[0] + CONTACT_DAMP * (p[0] - q[0]),
        q[1] + CONTACT_DAMP * (p[1] - q[1]),
        q[2] + CONTACT_DAMP * (p[2] - q[2]),
    ];
    (p, q)
}

/// Carries cloth a newly installed field has swallowed back out onto its
/// surface, without stirring it.
///
/// A body that grew leaves particles under its skin. Left for the ordinary
/// contact solve, each would be pushed the whole way out inside one substep
/// and [`super::solver::substep`] would derive a velocity from that jump, so
/// the garment would appear to be flung off the body it was resting on. Here
/// `p` and `q` move by the same vector, and PBD reads velocity as their
/// difference: the cloth arrives on the new surface carrying exactly the
/// motion it already had.
///
/// A particle deeper in than the band reaches has no gradient to follow —
/// out there the field is saturated flat — so it is left where it is and the
/// stretch constraints draw it out through the neighbours that did move.
pub fn lift_out_of(sdf: &SdfGrid, state: &mut State) {
    let eps = sdf.cell * 0.5;
    for i in 0..state.len() {
        let (x, y, z) = (state.px[i], state.py[i], state.pz[i]);
        let d = sdf.sample(x, y, z);
        if d >= 0.0 {
            continue;
        }
        let gx = sdf.sample(x + eps, y, z) - d;
        let gy = sdf.sample(x, y + eps, z) - d;
        let gz = sdf.sample(x, y, z + eps) - d;
        let square = gx * gx + gy * gy + gz * gz;
        if square <= 0.0 {
            continue;
        }
        let push = -d / square.sqrt();
        let (sx, sy, sz) = (gx * push, gy * push, gz * push);
        state.px[i] += sx;
        state.qx[i] += sx;
        state.py[i] += sy;
        state.qy[i] += sy;
        state.pz[i] += sz;
        state.qz[i] += sz;
    }
}
