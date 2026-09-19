use super::*;
use crate::xpbd::contact::Floor;
use crate::xpbd::metrics::position_hash;

/// Simulated seconds per substep, as the engine runs it.
const DT: f32 = 1.0 / 600.0;

/// Substeps run before the bits are read.
const STEPS: usize = 240;

/// Where the sheet is let go, in metres.
const RELEASE: f32 = 0.30;

/// A square of cloth above the origin, its stretch constraints, and a field
/// small enough to stay out of the way.
///
/// Scene enough that a pass which changed anything would show in the hash:
/// every particle moves, every constraint pulls, and the order they are
/// solved in is the order the goldens are taken in.
fn scene() -> (State, DistanceConstraints, SdfGrid) {
    const W: usize = 12;
    const SPACING: f32 = 0.02;
    let mut state = State::new(W * W);
    for j in 0..W {
        for i in 0..W {
            let v = j * W + i;
            state.px[v] = i as f32 * SPACING - 0.11;
            state.py[v] = RELEASE;
            state.pz[v] = j as f32 * SPACING - 0.11;
        }
    }
    let mut cons = DistanceConstraints::default();
    let link = |a: usize, b: usize, cons: &mut DistanceConstraints| {
        cons.a.push(a as u32);
        cons.b.push(b as u32);
        cons.rest.push(SPACING);
        cons.compliance.push(1.0e-8);
    };
    for j in 0..W {
        for i in 0..W {
            let v = j * W + i;
            if i + 1 < W {
                link(v, v + 1, &mut cons);
            }
            if j + 1 < W {
                link(v, v + W, &mut cons);
            }
        }
    }
    let sdf = SdfGrid::sphere(8, 0.05, [-0.2, -0.2, -0.2], [0.0, 0.0, 0.0], 0.03);
    (state, cons, sdf)
}

/// The lowest particle, in metres.
fn lowest(state: &State) -> f32 {
    state.py.iter().copied().fold(f32::MAX, f32::min)
}

/// A floor the cloth never reaches changes not one bit.
///
/// This is the whole promise the goldens rest on. The floor is a pass that a
/// scene without one does not run, so a scene with a plane far below the
/// cloth must produce the very same positions as a scene with no plane at
/// all — not positions that round to them. If this ever fails, every drape
/// golden is about to move for a scene nothing was standing on.
#[test]
fn a_floor_nothing_reaches_changes_no_bits() {
    let seams = Seams::default();
    let (mut without, cons, sdf) = scene();
    let mut with = without.clone();
    for _ in 0..STEPS {
        substep(&mut without, &cons, &seams, &Stage::around(&sdf), None, DT);
        let deep = Stage::around(&sdf).on(Floor::at(-100.0));
        substep(&mut with, &cons, &seams, &deep, None, DT);
    }
    assert!(lowest(&without) > -100.0, "the cloth never reached it");
    assert_eq!(
        position_hash(&without),
        position_hash(&with),
        "a floor out of reach is not a floor that rounds: it is no floor"
    );
}

/// And a floor the cloth does reach stops it.
///
/// The other half of the pair: the pass above is inert only because nothing
/// touched it, not because it does nothing.
#[test]
fn a_floor_the_cloth_reaches_holds_it_up() {
    let seams = Seams::default();
    let plane = 0.20;
    let (mut state, cons, sdf) = scene();
    let ground = Stage::around(&sdf).on(Floor::at(plane));
    for _ in 0..STEPS {
        substep(&mut state, &cons, &seams, &ground, None, DT);
    }
    assert!(
        lowest(&state) >= plane,
        "no particle went through the floor: {} under {plane}",
        lowest(&state)
    );
    assert!(
        lowest(&state) < RELEASE,
        "and it did come down to it: {}",
        lowest(&state)
    );
}

/// Without a floor the same sheet is still falling, which is what says the
/// test above measured a floor rather than a sheet that stops on its own.
#[test]
fn the_same_sheet_without_a_floor_is_still_falling() {
    let seams = Seams::default();
    let (mut state, cons, sdf) = scene();
    for _ in 0..STEPS {
        substep(&mut state, &cons, &seams, &Stage::around(&sdf), None, DT);
    }
    assert!(
        lowest(&state) < 0.20,
        "nothing stopped it: {}",
        lowest(&state)
    );
}

/// A weightless stage is the pull turned off and nothing else.
///
/// The sheet is let go flat, so every edge of it is horizontal and no
/// correction the constraints make has a vertical part: with nothing pulling,
/// each particle stays at the very height it started at, to the bit. The same
/// stage under ordinary gravity brings the sheet down, which is what says
/// this measured the pull rather than a scene that never moves.
#[test]
#[allow(
    clippy::float_cmp,
    reason = "nothing was added to these heights, so they are the bits they were given"
)]
fn a_weightless_stage_leaves_the_cloth_at_the_height_it_was_let_go() {
    let seams = Seams::default();
    let (mut hanging, cons, sdf) = scene();
    let mut falling = hanging.clone();
    for _ in 0..STEPS {
        substep(
            &mut hanging,
            &cons,
            &seams,
            &Stage::around(&sdf).weightless(),
            None,
            DT,
        );
        substep(&mut falling, &cons, &seams, &Stage::around(&sdf), None, DT);
    }
    assert!(
        hanging.py.iter().all(|&y| y == RELEASE),
        "nothing pulled on it: {}",
        lowest(&hanging)
    );
    assert!(lowest(&falling) < RELEASE, "and the same scene does fall");
}

/// What a band held in the constraint set is worth, and what decides it.
mod held;
