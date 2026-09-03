use toile_sim::xpbd::{self, Seams};

use crate::demo;

/// Drapes the demo bodice, moves the shoulder point 2 cm, drapes again, and
/// hashes the result.
///
/// Counted in substeps against the scalar reference solver: no wall clock, no
/// threads, so the answer is a property of the code rather than of the machine.
/// CI asserts it against one constant on macOS ARM and Linux x86 at once,
/// which is also what tests cross-architecture bit-exactness.
pub fn drape_bodice_hash() -> u64 {
    const DT: f32 = 1.0 / 600.0;
    const SUBSTEPS: usize = 600;

    let no_seams = Seams::default();
    let mut contour = demo::bodice_contour();
    let mut pipe = demo::pipeline(&contour);
    let mut state = demo::drop_state(&pipe);
    let mut cons = pipe.constraints(1.0e-8);
    let sdf = demo::avatar_sdf();

    for _ in 0..SUBSTEPS {
        xpbd::substep(&mut state, &cons, &no_seams, &sdf, DT);
    }
    contour[demo::SHOULDER_POINT][0] += 0.02;
    cons.rest.copy_from_slice(pipe.derive(&contour));
    for _ in 0..SUBSTEPS {
        xpbd::substep(&mut state, &cons, &no_seams, &sdf, DT);
    }
    xpbd::position_hash(&state)
}
