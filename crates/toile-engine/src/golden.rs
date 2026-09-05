use toile_sim::xpbd::{self, Seams};

use crate::demo;
use crate::draft::{Draft, block};

/// The offset basis and the prime of 64-bit FNV-1a, the same mix the solver's
/// position hash uses. One arithmetic per file keeps two goldens comparable.
const FNV_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0100_0000_01b3;

/// Flattens the base block's front and hashes the bits of the line it draws.
///
/// One level below the drape golden and far cheaper: no solver, no mesh, just
/// the document resolved and its curves cut into points. A drift in `kurbo`, in
/// the formula evaluator or in the platform's floating point moves this hash
/// before it can reach a rest state, which is where it would be hard to read.
///
/// # Panics
/// If the block the crate ships stops resolving to a contour.
pub fn flatten_front_hash() -> u64 {
    let draft = Draft::from_doc(block::trouser_front()).expect("the block resolves");
    let piece = draft
        .doc()
        .piece_named(block::FRONT)
        .expect("the block draws one piece");
    let mut h = FNV_BASIS;
    for [x, y] in draft.flat_cm(piece) {
        for bits in [x.to_bits(), y.to_bits()] {
            h = (h ^ bits).wrapping_mul(FNV_PRIME);
        }
    }
    h
}

/// Drapes the demo bodice, moves the shoulder point 2 cm, drapes again, and
/// hashes the result.
///
/// Counted in substeps against the scalar reference solver: no wall clock, no
/// threads, so the answer is a property of the code rather than of the machine.
/// CI asserts it against one constant on macOS ARM and Linux x86 at once,
/// which is also what tests cross-architecture bit-exactness.
///
/// # Panics
/// If the scene it builds itself stops being a contour the mesher accepts.
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
    let rests = pipe
        .derive(&contour)
        .expect("the golden moves a point, never the node count");
    cons.rest.copy_from_slice(rests);
    for _ in 0..SUBSTEPS {
        xpbd::substep(&mut state, &cons, &no_seams, &sdf, DT);
    }
    xpbd::position_hash(&state)
}
