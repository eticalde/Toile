use super::metrics::kinetic_energy;
use super::state::State;

/// Zeroes every velocity.
pub fn zero_velocities(state: &mut State) {
    state.vx.fill(0.0);
    state.vy.fill(0.0);
    state.vz.fill(0.0);
}

/// Provot kinetic damping: at each peak of kinetic energy, every velocity is
/// zeroed, so the cloth settles into static equilibrium instead of swinging.
///
/// This is the explicit convergence mechanism — without it a drape oscillates
/// for far longer than the interaction budget allows. Deterministic.
#[derive(Debug, Clone)]
pub struct KineticDamper {
    previous: f32,
    rising: bool,
}

impl Default for KineticDamper {
    fn default() -> Self {
        Self::new()
    }
}

impl KineticDamper {
    /// A damper that has seen no energy yet.
    pub fn new() -> Self {
        Self {
            previous: f32::MAX,
            rising: false,
        }
    }

    /// Forgets the energy history. Call after an edit: the previous peak says
    /// nothing about the new rest state.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Call once per substep. Returns the kinetic energy just measured.
    pub fn observe(&mut self, state: &mut State) -> f32 {
        let e = kinetic_energy(state);
        if e > self.previous {
            self.rising = true;
        } else if self.rising {
            zero_velocities(state);
            self.rising = false;
        }
        self.previous = e;
        e
    }
}
