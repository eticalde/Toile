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

/// Lofts the reference mannequin and hashes the bits of its mesh.
///
/// One level below any drape: no solver and no GPU, and — like
/// `flatten_front_hash` — it touches only `+ - * / sqrt` and f64 literals, so
/// it is bit-identical on macOS ARM and Linux x86. A drift in the loft math
/// moves it before it can reach a mesh nobody can read. Take the new value from
/// the assertion and commit it in the same change, saying why.
pub fn body_mesh_hash() -> u64 {
    let mesh = crate::body::body_from_measures(&crate::body::default_measures());
    let mut h = FNV_BASIS;
    for f in mesh.positions.iter().chain(&mesh.normals) {
        h = (h ^ u64::from(f.to_bits())).wrapping_mul(FNV_PRIME);
    }
    for &i in &mesh.indices {
        h = (h ^ u64::from(i)).wrapping_mul(FNV_PRIME);
    }
    h
}

/// Loads the neutral Anny body and hashes the bits of its mesh, the same way
/// [`body_mesh_hash`] hashes the loft.
///
/// The asset is baked once and committed (`crates/toile-anny/assets/body.bin`);
/// this only decodes it and computes normals, both in the `+ - * / sqrt`
/// regime, so the result is bit-identical on macOS ARM and Linux x86. It moves
/// only when the asset is re-baked or the normal computation changes — take
/// the new value from the assertion and commit it in the same change, saying
/// why.
pub fn anny_mesh_hash() -> u64 {
    let mesh = toile_anny::body_mesh();
    let mut h = FNV_BASIS;
    for f in mesh.positions.iter().chain(&mesh.normals) {
        h = (h ^ u64::from(f.to_bits())).wrapping_mul(FNV_PRIME);
    }
    for &i in &mesh.indices {
        h = (h ^ u64::from(i)).wrapping_mul(FNV_PRIME);
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
