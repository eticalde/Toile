use super::state::{Seams, State};

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

/// The widest a sewn pair stands open, in metres; zero when nothing is sewn.
///
/// The widest and not the mean, because what a caller wants to know is
/// whether the product is shut, and a garment with one seam still gaping is
/// not shut however well the rest of it came together.
pub fn seam_gap(state: &State, seams: &Seams) -> f32 {
    let mut worst = 0.0f32;
    for k in 0..seams.len() {
        let (ia, ib) = (seams.a[k] as usize, seams.b[k] as usize);
        let dx = state.px[ib] - state.px[ia];
        let dy = state.py[ib] - state.py[ia];
        let dz = state.pz[ib] - state.pz[ia];
        worst = worst.max(dx * dx + dy * dy + dz * dz);
    }
    worst.sqrt()
}

/// Largest particle speed, in metres per second.
///
/// Not what sleeping is decided on: that is mean kinetic energy per vertex,
/// which one loose vertex fluttering cannot hold above the threshold.
pub fn max_speed(state: &State) -> f32 {
    let mut m = 0.0f32;
    for i in 0..state.len() {
        let v2 = state.vx[i] * state.vx[i] + state.vy[i] * state.vy[i] + state.vz[i] * state.vz[i];
        m = m.max(v2);
    }
    m.sqrt()
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::float_cmp,
        reason = "a distance between two placed particles is the distance it was placed at"
    )]

    use super::*;

    /// Four particles on the x axis at 0, 1, 2 and 6 metres.
    fn spread() -> State {
        let mut state = State::new(4);
        for (i, x) in [0.0, 1.0, 2.0, 6.0].into_iter().enumerate() {
            state.px[i] = x;
        }
        state
    }

    /// One pair sewn between the two vertices named.
    fn sewn(pairs: &[(u32, u32)]) -> Seams {
        Seams {
            a: pairs.iter().map(|&(a, _)| a).collect(),
            b: pairs.iter().map(|&(_, b)| b).collect(),
            ..Seams::default()
        }
    }

    /// A product with nothing sewn stands nothing apart, which is what makes a
    /// lone panel fall from its first substep rather than wait to be closed.
    #[test]
    fn a_product_with_nothing_sewn_reads_no_gap_at_all() {
        assert_eq!(seam_gap(&spread(), &Seams::default()), 0.0);
    }

    /// The widest and not the mean: a garment with one seam still gaping is
    /// not shut however well the rest of it came together.
    #[test]
    fn the_widest_pair_is_the_answer_and_not_the_mean_of_them() {
        let state = spread();
        assert_eq!(seam_gap(&state, &sewn(&[(0, 1)])), 1.0);
        assert_eq!(seam_gap(&state, &sewn(&[(0, 1), (2, 3)])), 4.0);
        assert_eq!(
            seam_gap(&state, &sewn(&[(2, 3), (0, 1)])),
            4.0,
            "and the order the pairs were concatenated in decides nothing"
        );
    }

    /// A pair standing in one place reads shut, and one parted along any axis
    /// reads how far it was parted — the reading the closing phase waits on.
    #[test]
    fn a_pair_that_has_come_together_reads_shut() {
        let mut state = State::new(2);
        for i in 0..2 {
            state.px[i] = 0.25;
            state.py[i] = -0.5;
            state.pz[i] = 1.0;
        }
        let seams = sewn(&[(0, 1)]);
        assert_eq!(seam_gap(&state, &seams), 0.0);
        state.py[1] += 0.25;
        assert_eq!(seam_gap(&state, &seams), 0.25);
        state.py[1] -= 0.25;
        state.pz[1] -= 0.5;
        assert_eq!(seam_gap(&state, &seams), 0.5);
    }
}
