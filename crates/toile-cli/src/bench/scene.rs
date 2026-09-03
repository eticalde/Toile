use toile_sim::xpbd::{self, DistanceConstraints, KineticDamper, SdfGrid, Seams, State};

/// Simulated seconds per substep: 60 Hz visual at ten substeps a frame.
pub const DT: f32 = 1.0 / 600.0;

/// Substeps between sleep checks, matching the engine's tick.
const TICK: usize = 10;

/// Mean kinetic energy per vertex below which a tick counts as quiet.
const QUIET_ENERGY: f32 = 2.0e-6;

/// Consecutive quiet ticks before the cloth is considered settled.
const QUIET_TICKS: u32 = 3;

/// A tiny deterministic PRNG (Knuth MMIX), so the benchmark needs no
/// dependency to be reproducible.
pub struct Lcg(pub u64);

impl Lcg {
    pub fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    pub fn below(&mut self, n: usize) -> usize {
        ((self.next() >> 33) as usize) % n
    }
}

pub fn shuffle<T>(v: &mut [T], rng: &mut Lcg) {
    for i in (1..v.len()).rev() {
        v.swap(i, rng.below(i + 1));
    }
}

/// Runs substeps until the cloth stays quiet for [`QUIET_TICKS`] ticks, or the
/// cap is reached. Returns the substeps taken.
///
/// Same criterion as the engine's sim thread, so a benchmark number means the
/// same thing a user would experience.
pub fn settle(
    state: &mut State,
    cons: &DistanceConstraints,
    seams: &Seams,
    sdf: &SdfGrid,
    max_steps: usize,
) -> usize {
    let mut seams = seams.clone();
    settle_with(state, cons, &mut seams, sdf, max_steps, |_, _| {})
}

/// [`settle`] with a hook run before each substep, for schedules that change
/// the scene as it converges — progressive sewing, for instance.
pub fn settle_with(
    state: &mut State,
    cons: &DistanceConstraints,
    seams: &mut Seams,
    sdf: &SdfGrid,
    max_steps: usize,
    mut before: impl FnMut(usize, &mut Seams),
) -> usize {
    let inv_n = 1.0 / state.len() as f32;
    let mut damper = KineticDamper::new();
    let mut quiet = 0u32;
    let mut steps = 0usize;
    while quiet < QUIET_TICKS && steps < max_steps {
        before(steps, seams);
        xpbd::substep(state, cons, seams, sdf, DT);
        steps += 1;
        let e = damper.observe(state);
        if steps.is_multiple_of(TICK) {
            if e * inv_n < QUIET_ENERGY {
                quiet += 1;
            } else {
                quiet = 0;
            }
        }
    }
    steps
}

/// Simulated seconds represented by a substep count.
pub fn seconds(steps: usize) -> f64 {
    steps as f64 * f64::from(DT)
}

pub fn avg(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

pub fn max(v: &[f64]) -> f64 {
    v.iter().copied().fold(0.0f64, f64::max)
}

/// Verdict string for a determinism comparison.
pub fn same_bits(a: u64, b: u64) -> &'static str {
    if a == b {
        "OK (bit-idéntico)"
    } else {
        "FALLÓ"
    }
}
