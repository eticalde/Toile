use super::super::touch::{PARTING_SPEED, RELAXATION};
use super::*;
use crate::xpbd::metrics::max_speed;

/// Height the pinned triangle below lies at, in metres.
const TABLE: f32 = 0.20;

/// One loose particle `up` metres over the middle of one pinned triangle.
///
/// The smallest scene with a contact in it and nothing else: no gravity, no
/// second contact to share the correction with, no cloth pulling back. What
/// one call of the response does is then what the particle is seen to do.
fn over_a_pinned_triangle(up: f32) -> (State, DistanceConstraints, Vec<u32>) {
    let mut state = State::new(4);
    for (v, at) in [[0.0, 0.0], [0.0, 0.03], [0.03, 0.0], [0.01, 0.01]]
        .iter()
        .enumerate()
    {
        state.px[v] = at[0];
        state.py[v] = TABLE;
        state.pz[v] = at[1];
    }
    state.py[3] += up;
    for v in 0..3 {
        state.inv_mass[v] = 0.0;
    }
    let tris = vec![0, 1, 2];
    let cons = stretch(&state, &tris);
    (state, cons, tris)
}

/// A deep overlap is walked out, where it used to be thrown out.
///
/// Resolved whole, this one is most of a centimetre inside one substep: six
/// metres a second, handed to a particle that was standing still.
#[test]
fn a_deep_overlap_is_walked_out_over_several_substeps() {
    let (mut state, cons, tris) = over_a_pinned_triangle(0.002);
    let mut layers = Layers::of(&tris, &cons, &Seams::default(), state.len());
    let thickness = layers.thickness();
    let (sdf, seams) = (nowhere(), Seams::default());
    let stage = Stage::around(&sdf).weightless();

    substep(&mut state, &cons, &seams, &stage, Some(&mut layers), DT);
    let rise = state.py[3] - TABLE - 0.002;
    assert!(
        (rise - PARTING_SPEED * DT).abs() < 1.0e-6,
        "one substep takes out the cap and no more: {rise}"
    );
    assert!(
        max_speed(&state) <= PARTING_SPEED,
        "and hands over no more speed than the cap: {}",
        max_speed(&state)
    );

    let mut taken = 1;
    while state.py[3] - TABLE < thickness && taken < STEPS {
        substep(&mut state, &cons, &seams, &stage, Some(&mut layers), DT);
        taken += 1;
    }
    assert!(taken > 2, "it was not thrown clear: {taken} substeps");
    assert!(taken < STEPS, "and it does come clear");
}

/// A shallow one gives up a share of itself, which is what keeps several
/// contacts on one particle from each undoing the others.
#[test]
fn a_shallow_overlap_gives_up_a_share_of_itself() {
    let (state, cons, tris) = over_a_pinned_triangle(0.0);
    let mut layers = Layers::of(&tris, &cons, &Seams::default(), state.len());
    let depth = 0.001;
    let mut state = state;
    state.py[3] += layers.thickness() - depth;
    let before = state.py[3];
    let (sdf, seams) = (nowhere(), Seams::default());
    let stage = Stage::around(&sdf).weightless();

    substep(&mut state, &cons, &seams, &stage, Some(&mut layers), DT);
    let rise = state.py[3] - before;
    assert!(
        (rise - RELAXATION * depth).abs() < 1.0e-6,
        "{rise} of a {depth} overlap"
    );
    assert_eq!(layers.contacts(), 1);
}
