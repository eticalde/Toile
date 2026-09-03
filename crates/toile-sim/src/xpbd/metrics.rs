use super::state::State;

/// FNV-1a over the bits of every position — the determinism golden.
///
/// Same scene and same substep count must give the same value, on every run
/// and on every architecture.
pub fn position_hash(state: &State) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |v: &[f32]| {
        for x in v {
            h = (h ^ u64::from(x.to_bits())).wrapping_mul(0x0100_0000_01b3);
        }
    };
    eat(&state.px);
    eat(&state.py);
    eat(&state.pz);
    h
}

/// Total kinetic energy at unit mass — the sensor kinetic damping watches.
pub fn kinetic_energy(state: &State) -> f32 {
    let mut e = 0.0f32;
    for i in 0..state.len() {
        e += state.vx[i] * state.vx[i] + state.vy[i] * state.vy[i] + state.vz[i] * state.vz[i];
    }
    e * 0.5
}

/// Largest particle speed — the convergence sensor for sleeping.
pub fn max_speed(state: &State) -> f32 {
    let mut m = 0.0f32;
    for i in 0..state.len() {
        let v2 = state.vx[i] * state.vx[i] + state.vy[i] * state.vy[i] + state.vz[i] * state.vz[i];
        m = m.max(v2);
    }
    m.sqrt()
}
