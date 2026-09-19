use super::super::*;
use super::{DT, STEPS};

/// Vertices in a ring of this many: the waistband the engine reads off a
/// skirt is about this size.
const RING: usize = 256;

/// What the ring's own edges are asked to rest at, as a share of the circle
/// they were laid on: a waistband's fifteen per cent.
const GATHERED: f32 = 0.85;

/// A ring of particles whose own edges ask to be [`GATHERED`] shorter, laced
/// across by chords that ask for the length they were laid at.
///
/// The cloth a waistband is written on, reduced to the argument: at every
/// vertex of the run, two edges pulling in and several more holding out. The
/// held edges are the ring's own, which are the first `RING` of the set.
fn laced(r: f32) -> (State, DistanceConstraints, Vec<u32>) {
    let mut state = State::new(RING);
    for i in 0..RING {
        let turn = std::f64::consts::TAU * i as f64 / RING as f64;
        state.px[i] = r * turn.cos() as f32;
        state.pz[i] = r * turn.sin() as f32;
    }
    let mut cons = DistanceConstraints::default();
    let link = |cons: &mut DistanceConstraints, a: usize, b: usize, share: f32| {
        let (dx, dz) = (state.px[b] - state.px[a], state.pz[b] - state.pz[a]);
        cons.a.push(a as u32);
        cons.b.push(b as u32);
        cons.rest.push((dx * dx + dz * dz).sqrt() * share);
        cons.compliance.push(1.0e-8);
    };
    for i in 0..RING {
        link(&mut cons, i, (i + 1) % RING, GATHERED);
    }
    for step in [2, 3, 4] {
        for i in 0..RING {
            link(&mut cons, i, (i + step) % RING, 1.0);
        }
    }
    let mut walk: Vec<u32> = (0..RING as u32).collect();
    walk.push(0);
    (state, cons, walk)
}

/// How far round a walk measures, in metres.
fn round(state: &State, walk: &[u32]) -> f32 {
    walk.windows(2)
        .map(|pair| {
            let (a, b) = (pair[0] as usize, pair[1] as usize);
            let (dx, dy, dz) = (
                state.px[b] - state.px[a],
                state.py[b] - state.py[a],
                state.pz[b] - state.pz[a],
            );
            (dx * dx + dy * dy + dz * dz).sqrt()
        })
        .sum()
}

/// Settles the laced ring with `passes` extra sweeps over its own edges at
/// `compliance`, and answers how far round it came to.
fn after(passes: u32, compliance: f32) -> f32 {
    let seams = Seams::default();
    let sdf = SdfGrid::sphere(8, 0.05, [-0.2, -0.2, -0.2], [0.0, 0.0, 0.0], 0.01);
    let stage = Stage::around(&sdf).weightless();
    let (mut state, mut cons, walk) = laced(0.14);
    cons.held = (0..RING as u32).collect();
    cons.held_passes = passes;
    for &e in &cons.held {
        cons.compliance[e as usize] = compliance;
    }
    for _ in 0..STEPS {
        substep(&mut state, &cons, &seams, &stage, None, DT);
    }
    round(&state, &walk)
}

/// Swept again, a held ring carries the vote; swept once, the cloth laced
/// across it wins and the ring hardly moves.
///
/// This is the whole argument for `DistanceConstraints::held`, and the reason
/// compliance could never have been it: every edge here runs at the same
/// stiffness, and what decides the answer is how many times the ring's own
/// edges are put. The answer is monotone in the sweeps, which is what lets a
/// person's number for how firm a band is mean something the solver can act on.
#[test]
fn a_ring_swept_again_carries_the_vote_and_more_sweeps_hold_it_tighter() {
    let (mut last, mut readings) = (f32::MAX, Vec::new());
    for passes in [0, 4, 16, 64] {
        let came = after(passes, 1.0e-8);
        readings.push((passes, came));
        assert!(
            came < last,
            "{passes} sweeps left {came}, no less than {last}"
        );
        last = came;
    }
    println!("laced ring, by sweeps: {readings:?}");
    let (loose, tight) = (readings[0].1, readings[3].1);
    assert!(
        tight < loose - 0.05,
        "and the difference is one a person would see: {tight} against {loose}"
    );
}

/// And once it is swept again, how stiff it is written at is felt too.
///
/// This is the half compliance could not buy on its own. A firmer band puts
/// more of its correction each time it is swept, so over a run of sweeps the
/// two compound and the settled ring answers the number a person set. The four
/// compliances here are the four the shipped rail writes, from its slack end
/// to its firm one, and what is asserted is a span rather than a threshold:
/// what a rail has to do is read differently along the whole of its length.
#[test]
fn a_held_rings_stiffness_is_felt_once_it_is_swept_again() {
    let (mut last, mut readings) = (f32::MAX, Vec::new());
    for compliance in [1.0e-4, 1.0e-5, 1.0e-6, 2.0e-7] {
        let came = after(64, compliance);
        readings.push((compliance, came));
        assert!(came < last, "{compliance} left {came}, no less than {last}");
        last = came;
    }
    println!("laced ring at 64 sweeps, along the rail: {readings:?}");
    let (slack, firm) = (readings[0].1, readings[3].1);
    assert!(
        slack > firm + 0.05,
        "and the two ends of the rail are a span apart: {slack} against {firm}"
    );
}

/// What that ring would come to with nothing laced across it, which is what
/// says the tests above measured a vote and not a ring that shrinks anyway.
#[test]
fn the_same_ring_unlaced_reaches_its_ratio_with_no_extra_sweep_at_all() {
    let seams = Seams::default();
    let sdf = SdfGrid::sphere(8, 0.05, [-0.2, -0.2, -0.2], [0.0, 0.0, 0.0], 0.01);
    let stage = Stage::around(&sdf).weightless();
    let (mut state, cons, walk) = laced(0.14);
    let bare = DistanceConstraints {
        a: cons.a[..RING].to_vec(),
        b: cons.b[..RING].to_vec(),
        rest: cons.rest[..RING].to_vec(),
        compliance: cons.compliance[..RING].to_vec(),
        ..DistanceConstraints::default()
    };
    let asked: f32 = bare.rest.iter().sum();
    for _ in 0..STEPS {
        substep(&mut state, &bare, &seams, &stage, None, DT);
    }
    let came = round(&state, &walk);
    assert!(
        (came - asked).abs() < 0.002,
        "nothing outvoting it, the ring reaches its ratio: {came} against {asked}"
    );
}
