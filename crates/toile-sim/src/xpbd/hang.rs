use super::state::State;

/// A run of cloth held at the height of one of the body's own rings.
///
/// The height alone, and not the circle that ring lies on. Measured, a run held
/// in to the ring's radius as well settles inside that radius everywhere — a
/// waist is an ellipse and the contact solve has the last word — so the radial
/// half spends every substep being undone, and what it costs while it does is
/// the cap: a cap small enough to leave a seam shut is too small to hold the
/// cloth up at all.
#[derive(Debug, Clone)]
pub struct Hung {
    /// The vertices of the combined state it holds.
    pub at: Vec<u32>,
    /// The height they are held at, in metres.
    pub height: f32,
    /// Most one substep may move one of them toward it, in metres.
    pub max_step: f32,
}

/// Pulls every hung vertex toward its own ring's height, by no more than the
/// cap it carries.
///
/// Hard rather than compliant, and the cap is what does the work. At the step a
/// drape runs at, one pass barely tells one stiffness from another — the
/// argument `solve_held` makes for sweeping held edges again instead of
/// stiffening them — so a pull that gets one correction a substep has nothing
/// to gain from a compliance, and all that is left to choose is how far that
/// correction may go. It is the caller's number because what it has to stay
/// under belongs to the body's field.
///
/// A run with nothing in it costs a substep nothing: no branch, no arithmetic,
/// and no bit of the state written. That is what lets a product with no anchor
/// drape as it always has.
pub(super) fn solve(state: &mut State, hung: &[Hung]) {
    for run in hung {
        for &v in &run.at {
            let i = v as usize;
            // A pinned particle is left where it is, as every other constraint
            // here leaves one: the ring does not outrank a hand holding cloth.
            if state.inv_mass[i] == 0.0 {
                continue;
            }
            let rise = run.height - state.py[i];
            state.py[i] += rise.clamp(-run.max_step, run.max_step);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How near a reading has to be to count as the same height, in metres:
    /// a thousandth of the cap these run at.
    const NEAR: f32 = 1.0e-4;

    /// Three particles, the middle one pinned, spread over a metre of height.
    fn state() -> State {
        let mut state = State::new(3);
        state.py = vec![1.0, 0.5, 0.0];
        state.inv_mass[1] = 0.0;
        state
    }

    fn ring(at: Vec<u32>) -> Hung {
        Hung {
            at,
            height: 0.5,
            max_step: 0.1,
        }
    }

    fn near(got: f32, want: f32) -> bool {
        (got - want).abs() < NEAR
    }

    /// The pull is toward the ring from either side of it, and never further
    /// than the cap in one substep.
    #[test]
    fn a_hung_vertex_moves_toward_the_ring_by_at_most_the_cap() {
        let mut state = state();
        let hung = [ring(vec![0, 2])];
        solve(&mut state, &hung);
        assert!(near(state.py[0], 0.9), "down from above: {}", state.py[0]);
        assert!(near(state.py[2], 0.1), "up from below: {}", state.py[2]);
        solve(&mut state, &hung);
        assert!(near(state.py[0], 0.8), "{}", state.py[0]);
        assert!(near(state.py[2], 0.2), "{}", state.py[2]);
        assert_eq!(state.px, vec![0.0; 3], "and nothing sideways");
        assert_eq!(state.pz, vec![0.0; 3]);
    }

    /// Once the run is at its ring it asks for nothing, so it does not
    /// overshoot and then come back.
    #[test]
    fn a_vertex_at_the_ring_is_left_where_it_is() {
        let mut state = state();
        state.py[0] = 0.5;
        solve(&mut state, &[ring(vec![0])]);
        assert!(near(state.py[0], 0.5), "{}", state.py[0]);
    }

    /// A pinned particle is not moved, and neither is one nothing names.
    #[test]
    fn a_pinned_vertex_and_an_unnamed_one_are_not_pulled() {
        let mut state = state();
        solve(&mut state, &[ring(vec![1])]);
        assert_eq!(state.py, vec![1.0, 0.5, 0.0]);
    }

    /// The reduction every drape golden stands on: with nothing hung, the pass
    /// writes no bit of the state.
    #[test]
    fn a_product_that_hangs_from_nothing_is_the_product_of_today() {
        let mut state = state();
        let before = state.py.clone();
        solve(&mut state, &[]);
        solve(&mut state, &[ring(Vec::new())]);
        assert_eq!(state.py, before);
    }
}
