#![allow(
    clippy::float_cmp,
    reason = "velocities that were zeroed carry an energy of exactly nothing"
)]

use toile_sim::xpbd;

use super::*;

/// A tick of simulated time, in seconds: ten substeps at 600 Hz.
const TICK: f32 = 1.0 / 60.0;

/// The mean kinetic energy per vertex a tick once had to stay under to count
/// as quiet, kept here as the reading the displacement test is held against.
const MEAN_ENERGY_ONCE_QUIET: f32 = 2.0e-6;

/// A garment of a thousand vertices lying along a line, going nowhere.
fn garment() -> State {
    let mut state = State::new(1000);
    for i in 0..state.len() {
        state.px[i] = i as f32 * 0.001;
    }
    state
}

/// Carries the first `hem` vertices down at `speed` for one tick, leaving the
/// velocity on them that such a fall would.
fn fall(state: &mut State, hem: usize, speed: f32) {
    for i in 0..hem {
        state.py[i] -= speed * TICK;
        state.vy[i] = -speed;
    }
}

/// Runs `ticks` ticks over which the first `hem` vertices keep falling, and
/// says whether the garment was asleep at the end of them.
fn asleep_after(ticks: u32, hem: usize, speed: f32) -> bool {
    let mut state = garment();
    let mut sleep = Sleep::default();
    for _ in 0..ticks {
        sleep.mark(&state);
        fall(&mut state, hem, speed);
        sleep.judge(&state);
    }
    sleep.asleep()
}

/// Runs `ticks` ticks over which nothing moves at all.
fn still(state: &State, sleep: &mut Sleep, ticks: u32) {
    for _ in 0..ticks {
        sleep.mark(state);
        sleep.judge(state);
    }
}

/// The failure the mean could not see: one vertex in a hundred still falling
/// at 15 mm/s while the rest lie still. Averaged over the garment that is
/// little more than half the energy a quiet tick was allowed, so it slept.
#[test]
fn a_hem_still_falling_keeps_awake_a_garment_the_mean_would_let_sleep() {
    let mut state = garment();
    fall(&mut state, 10, 0.015);
    let mean = xpbd::kinetic_energy(&state) / state.len() as f32;
    assert!(
        mean < MEAN_ENERGY_ONCE_QUIET,
        "the scene is one the mean called quiet: {mean}"
    );
    assert!(!asleep_after(10 * QUIET_TICKS_TO_SLEEP, 10, 0.015));
}

/// The other failure: kinetic damping zeroes every velocity at an energy
/// peak, so a tick ending on one reads an energy of exactly nothing while the
/// cloth has plainly travelled. Where it got to is not something a zeroed
/// velocity can take back.
#[test]
fn a_tick_that_ends_on_a_damping_zero_is_not_a_quiet_one() {
    let mut state = garment();
    let mut sleep = Sleep::default();
    for _ in 0..10 * QUIET_TICKS_TO_SLEEP {
        sleep.mark(&state);
        fall(&mut state, 1000, 0.015);
        xpbd::zero_velocities(&mut state);
        assert_eq!(xpbd::kinetic_energy(&state), 0.0);
        sleep.judge(&state);
    }
    assert!(!sleep.asleep());
}

/// A garment going nowhere sleeps on the dwell and not a tick before it.
#[test]
fn a_still_garment_sleeps_once_it_has_been_still_for_the_whole_dwell() {
    assert!(!asleep_after(QUIET_TICKS_TO_SLEEP - 1, 0, 0.0));
    assert!(asleep_after(QUIET_TICKS_TO_SLEEP, 0, 0.0));
}

/// One loose vertex fluttering for ever must not cost a core for ever, and a
/// handful in a thousand is where that stops being one loose vertex.
#[test]
fn a_few_vertices_that_never_settle_do_not_keep_the_rest_awake() {
    assert!(asleep_after(QUIET_TICKS_TO_SLEEP, MOVING_PER_MILLE, 0.030));
    assert!(!asleep_after(
        10 * QUIET_TICKS_TO_SLEEP,
        MOVING_PER_MILLE + 1,
        0.030
    ));
}

/// The dwell is a run and not a tally: one tick of motion starts it again, so
/// a lull between two slips is never added to the lull after them.
#[test]
fn one_tick_of_motion_starts_the_dwell_again() {
    let mut state = garment();
    let mut sleep = Sleep::default();
    still(&state, &mut sleep, QUIET_TICKS_TO_SLEEP - 1);
    sleep.mark(&state);
    fall(&mut state, 1000, 0.015);
    sleep.judge(&state);
    still(&state, &mut sleep, QUIET_TICKS_TO_SLEEP - 1);
    assert!(!sleep.asleep(), "the two lulls were not added together");
    still(&state, &mut sleep, 1);
    assert!(sleep.asleep());
}

/// Sleep is a pause: waking forgets the dwell already served, so the drape
/// has to come to rest again rather than sleep on the rest it had.
#[test]
fn waking_is_at_once_and_the_dwell_is_served_again() {
    let state = garment();
    let mut sleep = Sleep::default();
    still(&state, &mut sleep, QUIET_TICKS_TO_SLEEP);
    assert!(sleep.asleep());
    sleep.wake();
    assert!(!sleep.asleep());
    still(&state, &mut sleep, 1);
    assert!(!sleep.asleep(), "one quiet tick after waking is not rest");
}

/// A drape that has blown up is not at rest, whatever a comparison with NaN
/// says: it stays awake rather than being put away as settled.
#[test]
fn a_position_gone_to_nan_counts_as_moving() {
    let mut state = garment();
    let mut sleep = Sleep::default();
    for _ in 0..QUIET_TICKS_TO_SLEEP {
        sleep.mark(&state);
        state.py.fill(f32::NAN);
        sleep.judge(&state);
    }
    assert!(!sleep.asleep());
}
