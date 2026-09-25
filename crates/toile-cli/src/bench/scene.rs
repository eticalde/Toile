use std::fmt;

use toile_engine::sync::Sleep;
use toile_sim::xpbd::{self, DistanceConstraints, KineticDamper, SdfGrid, Seams, Stage, State};

/// Simulated seconds per substep: 60 Hz visual at ten substeps a frame.
pub const DT: f32 = 1.0 / 600.0;

/// Substeps between two sleep judgements, matching the engine's tick.
const TICK: usize = 10;

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

/// What a drape run came to.
///
/// It carries whether the drape actually slept, because a run that used up its
/// cap measured nothing: printing that cap as a number is how a bench comes to
/// report its own timeout as a result.
pub struct Settled {
    steps: usize,
    asleep: bool,
}

impl Settled {
    /// Whether the drape reached sleep before the cap ran out.
    pub fn asleep(&self) -> bool {
        self.asleep
    }

    /// Simulated seconds the run covered.
    pub fn seconds(&self) -> f64 {
        seconds(self.steps)
    }
}

impl fmt::Display for Settled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:7.2} s de sim", self.seconds())?;
        if !self.asleep {
            write!(f, " ¡TOPE! la tela seguía moviéndose")?;
        }
        Ok(())
    }
}

/// Runs substeps until the drape goes to sleep, or the cap runs out.
///
/// The sim thread's rule, imported rather than restated: a number a bench
/// prints then means what a person waiting in front of the application means
/// by it, including the several seconds of quiet the rule wants before it will
/// call a drape finished.
pub fn settle(
    state: &mut State,
    cons: &DistanceConstraints,
    seams: &Seams,
    sdf: &SdfGrid,
    max_steps: usize,
) -> Settled {
    let mut seams = seams.clone();
    settle_with(state, cons, &mut seams, sdf, max_steps, |_, _| {})
}

/// [`settle`] with a hook run before each substep, for schedules that change
/// the scene as it converges — progressive sewing, for instance.
///
/// The stage carries no ground, as every drape golden's does. A panel with
/// nothing holding it therefore slides off the sphere and falls for as long as
/// anything integrates it, gaining speed each substep, and what ends such a run
/// is the cap rather than the drape — which is why [`Settled`] says which of
/// the two it was.
pub fn settle_with(
    state: &mut State,
    cons: &DistanceConstraints,
    seams: &mut Seams,
    sdf: &SdfGrid,
    max_steps: usize,
    mut before: impl FnMut(usize, &mut Seams),
) -> Settled {
    let mut damper = KineticDamper::new();
    let mut sleep = Sleep::default();
    let mut steps = 0usize;
    // In whole ticks, because the rule reads how far a vertex travelled across
    // one and there is nothing to judge in the middle of it.
    while !sleep.asleep() && steps + TICK <= max_steps {
        sleep.mark(state);
        for _ in 0..TICK {
            before(steps, seams);
            xpbd::substep(state, cons, seams, &Stage::around(sdf), None, DT);
            steps += 1;
            damper.observe(state);
        }
        sleep.judge(state);
    }
    Settled {
        steps,
        asleep: sleep.asleep(),
    }
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
