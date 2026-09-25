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

/// Halvings [`retreat`] spends narrowing where the field's band ends.
///
/// A halving and not a march, because each one doubles the precision instead
/// of adding a fixed stride: eight of them place the contact within a 256th of
/// the step that overshot, whatever its length. A stride has to be chosen
/// against the longest step it will ever be asked about, and on the shortest
/// one it is then the whole of it.
const RETREATS: u32 = 8;

/// The field's gradient at a point, by finite differences over `eps`.
///
/// Undivided, so its magnitude is the field's slope times `eps` rather than
/// the slope itself. Every caller here wants only its direction and whether
/// there is one at all, and the division would buy neither.
#[inline]
fn slope(sdf: &SdfGrid, eps: f32, p: [f32; 3], d: f32) -> [f32; 3] {
    [
        sdf.sample(p[0] + eps, p[1], p[2]) - d,
        sdf.sample(p[0], p[1] + eps, p[2]) - d,
        sdf.sample(p[0], p[1], p[2] + eps) - d,
    ]
}

/// The deepest place found on the step from `q` to `p` that the field still
/// has a gradient at: where it is, what it reads there, and its slope.
///
/// A constraint can carry a particle further in one substep than the band the
/// field was baked to reaches, and out there the field is saturated flat. The
/// step itself is the way back: a particle that ended past the band and began
/// the substep where the field could still be read crossed the skin on the
/// way, so the segment it travelled holds a last place with a gradient, and
/// halving toward `q` narrows onto it.
///
/// `None` when `q` is as flat as `p`. A particle already past the band before
/// the step, or one thrown in from further out than the band reaches on the
/// outside, has nothing on this step to be carried back along, and that is the
/// one the stretch constraints have to answer for.
fn retreat(sdf: &SdfGrid, eps: f32, p: [f32; 3], q: [f32; 3]) -> Option<([f32; 3], f32, [f32; 3])> {
    let read = |t: f32| {
        let at = [0, 1, 2].map(|k| p[k] + (q[k] - p[k]) * t);
        let d = sdf.sample(at[0], at[1], at[2]);
        (at, d, slope(sdf, eps, at, d))
    };
    let flat = |g: [f32; 3]| g[0] * g[0] + g[1] * g[1] + g[2] * g[2] <= 0.0;
    let mut found = read(1.0);
    if flat(found.2) {
        return None;
    }
    let (mut deep, mut back) = (0.0f32, 1.0f32);
    for _ in 0..RETREATS {
        // The bracket's middle, written as a step along it rather than as a
        // mean: only `+ - * /` reaches the geometry, and `f32::midpoint` is
        // std's arithmetic rather than that.
        let half = deep + (back - deep) * 0.5;
        let at = read(half);
        if flat(at.2) {
            deep = half;
        } else {
            back = half;
            found = at;
        }
    }
    // Only ever a point under the skin: one the step had not reached the body
    // at is a particle for the passes after this to place, not a contact.
    (found.1 < 0.0).then_some(found)
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
    let mut d = sdf.sample(p[0], p[1], p[2]);
    if d >= 0.0 {
        return (p, q);
    }
    let mut at = p;
    let mut g = slope(sdf, eps, at, d);
    // Deeper in than the band reaches, the field is saturated flat and there
    // is no normal to be had where the particle stands. The push along it
    // would move the particle nowhere, the friction below would take away
    // every bit of tangential motion against that same nothing, and the
    // damping would stop it dead out there — so the contact is taken where
    // this step last crossed a field that had a normal. Where there is no
    // such place, the particle is left for the stretch constraints to draw out
    // through its neighbours, as [`lift_out_of`] leaves it.
    let mut square = g[0] * g[0] + g[1] * g[1] + g[2] * g[2];
    if square <= 0.0 {
        let Some((there, depth, crossed)) = retreat(sdf, eps, at, q) else {
            return (p, q);
        };
        (at, d, g) = (there, depth, crossed);
        square = g[0] * g[0] + g[1] * g[1] + g[2] * g[2];
    }
    let (gx, gy, gz) = (g[0], g[1], g[2]);
    let glen = square.sqrt().max(1.0e-9);
    let push = -d / glen;
    let mut p = [at[0] + gx * push, at[1] + gy * push, at[2] + gz * push];

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
        let [gx, gy, gz] = slope(sdf, eps, [x, y, z], d);
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
