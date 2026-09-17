use std::collections::VecDeque;

use super::Lattice;

/// Which side of the skin a flood decided one untouched sample is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Claim {
    /// Not reached by either flood yet.
    None,
    /// Air.
    Outside,
    /// Body.
    Inside,
}

/// Gives every sample the band never reached a saturated `±band`, flooding
/// each untouched region from the band that walls it in.
///
/// Reach is the wrong question to ask. A pocket whose mouth is narrower than
/// the band is sealed off from the box's rim by the band itself, so "could
/// not be reached from the outside" catches trapped air as well as flesh.
/// What does hold is the sign: two untouched samples one cell apart cannot
/// straddle the skin, because the true distance moves by at most that cell
/// between them and both are further out than the band, so one of them would
/// have to be in it. Every untouched region therefore carries one sign
/// throughout, and so do the band samples along its border — seeding each
/// flood from the untouched neighbours of a touched sample of its own sign
/// partitions the untouched set exactly, with no rim to special-case.
///
/// The solver never reads a magnitude out here, only a sign: it asks about
/// particles it is holding against the skin.
pub(super) fn saturate(lattice: &Lattice, nearest: &[f32], data: &mut [f32]) {
    let band = lattice.band as f32;
    let untouched = |slot: usize| nearest[slot] == f32::INFINITY;
    let mut claim = vec![Claim::None; lattice.len()];
    let mut queue = VecDeque::new();

    let [nx, ny, nz] = lattice.dims;
    for k in 0..nz {
        for j in 0..ny {
            for i in 0..nx {
                let slot = lattice.index(i, j, k);
                if untouched(slot) {
                    continue;
                }
                let side = if data[slot] < 0.0 {
                    Claim::Inside
                } else {
                    Claim::Outside
                };
                spread(lattice, [i, j, k], side, &untouched, &mut claim, &mut queue);
            }
        }
    }

    while let Some(n) = queue.pop_front() {
        let side = claim[lattice.index(n[0], n[1], n[2])];
        spread(lattice, n, side, &untouched, &mut claim, &mut queue);
    }

    for slot in 0..lattice.len() {
        if !untouched(slot) {
            continue;
        }
        debug_assert_ne!(
            claim[slot],
            Claim::None,
            "an untouched region with no band of its own to take a sign from"
        );
        data[slot] = if claim[slot] == Claim::Inside {
            -band
        } else {
            band
        };
    }
}

/// Claims every untouched sample one step from `n` for `side`.
fn spread(
    lattice: &Lattice,
    n: [usize; 3],
    side: Claim,
    untouched: &impl Fn(usize) -> bool,
    claim: &mut [Claim],
    queue: &mut VecDeque<[usize; 3]>,
) {
    for next in steps(lattice, n).into_iter().flatten() {
        let slot = lattice.index(next[0], next[1], next[2]);
        if !untouched(slot) {
            continue;
        }
        if claim[slot] == Claim::None {
            claim[slot] = side;
            queue.push_back(next);
        } else {
            debug_assert_eq!(
                claim[slot], side,
                "one untouched region reached from both sides of the skin"
            );
        }
    }
}

/// The six samples one step from `n` that are inside the lattice.
fn steps(lattice: &Lattice, n: [usize; 3]) -> [Option<[usize; 3]>; 6] {
    let [i, j, k] = n;
    let [nx, ny, nz] = lattice.dims;
    [
        (i > 0).then(|| [i - 1, j, k]),
        (i + 1 < nx).then(|| [i + 1, j, k]),
        (j > 0).then(|| [i, j - 1, k]),
        (j + 1 < ny).then(|| [i, j + 1, k]),
        (k > 0).then(|| [i, j, k - 1]),
        (k + 1 < nz).then(|| [i, j, k + 1]),
    ]
}
