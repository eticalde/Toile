use std::collections::VecDeque;

use super::Lattice;

/// Gives every sample the band never reached a saturated `±band`: flesh
/// wherever a flood from the band's inside samples arrives, air elsewhere.
///
/// Reach from the box's rim is the wrong question to ask. A pocket whose
/// mouth is narrower than the band is sealed off from the rim by the band
/// itself, so "could not be reached from the outside" catches trapped air as
/// well as flesh. What does hold is that an untouched sample and a sample
/// one cell from it cannot straddle a sheet of the skin: the true distance
/// moves by at most that cell between them, so the untouched one would have
/// to be in the band. The skin therefore wraps an untouched region, and every
/// band sample along its border, the same number of times throughout.
///
/// The solver never reads a magnitude out here, only a sign: it asks about
/// particles it is holding against the skin.
pub(super) fn saturate(lattice: &Lattice, nearest: &[f32], data: &mut [f32]) {
    let band = lattice.band as f32;
    let untouched = |slot: usize| nearest[slot] == f32::INFINITY;
    let mut flesh = vec![false; lattice.len()];
    let mut queue = VecDeque::new();

    // Only an inside reading seeds a flood, because it is the only one a
    // body that passes through itself leaves trustworthy. A sample behind
    // its nearest sheet is wrapped once more than whatever lies in front of
    // that sheet, and a skin with no part turned inside out wraps nothing
    // fewer than zero times: it is flesh, and so is the region it borders. A
    // sample in front of its nearest sheet reads air whether that sheet is
    // the skin or a sheet of it buried in flesh, as where two buttocks meet.
    // So a region with both readings on its border is flesh, and were it
    // not, flesh is still the safer mistake in a field cloth collides with:
    // cloth is pushed out of doubtful space rather than left to sit in it.
    // Where the skin crosses nothing, no region has both and this decides
    // nothing.
    let [nx, ny, nz] = lattice.dims;
    for k in 0..nz {
        for j in 0..ny {
            for i in 0..nx {
                let slot = lattice.index(i, j, k);
                if !untouched(slot) && data[slot] < 0.0 {
                    spread(lattice, [i, j, k], &untouched, &mut flesh, &mut queue);
                }
            }
        }
    }
    while let Some(n) = queue.pop_front() {
        spread(lattice, n, &untouched, &mut flesh, &mut queue);
    }

    for slot in 0..lattice.len() {
        if untouched(slot) {
            data[slot] = if flesh[slot] { -band } else { band };
        }
    }
}

/// Marks every untouched sample one step from `n` as flesh.
///
/// A sample is queued once, when it is first marked, and whichever seed gets
/// to it first changes nothing: what comes out is the set the seeds can
/// reach, and a set has no scan order in it.
fn spread(
    lattice: &Lattice,
    n: [usize; 3],
    untouched: &impl Fn(usize) -> bool,
    flesh: &mut [bool],
    queue: &mut VecDeque<[usize; 3]>,
) {
    for next in steps(lattice, n).into_iter().flatten() {
        let slot = lattice.index(next[0], next[1], next[2]);
        if untouched(slot) && !flesh[slot] {
            flesh[slot] = true;
            queue.push_back(next);
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
