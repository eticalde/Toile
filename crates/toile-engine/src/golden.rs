/// The one scene a seam and a body are pinned in together.
mod sewn;

use toile_sim::xpbd::{self, SdfGrid, Seams, Stage};

pub use self::sewn::drape_sewn_hash;
use crate::demo;
use crate::draft::{Draft, block};

/// The offset basis and the prime of 64-bit FNV-1a, the same mix the solver's
/// position hash uses. One arithmetic per crate keeps two hashes comparable,
/// which is why the body cache's key is mixed with these and not its own.
pub(crate) const FNV_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
pub(crate) const FNV_PRIME: u64 = 0x0100_0000_01b3;

/// Flattens the base block's front and hashes the bits of the line it draws.
///
/// No solver and no mesh: just the document resolved and its curves cut into
/// points.
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

/// The body both Anny goldens below are taken against: male (`gender: 0.0`,
/// Anny's own convention), age parameter `0.8` (Anny's raw unit, not years —
/// a blend leaning toward `old`, between the `young` anchor at 2/3 and the
/// `old` anchor at 1.0), every other input at Anny's neutral `0.5`.
///
/// Deliberately not the all-`0.5` default: moving gender and age off their
/// symmetric midpoints means a mistake in an interpolation direction cannot
/// hide behind symmetry the way it could at the bare template. One constant
/// rather than one literal per golden, because the measures golden only
/// discriminates a ring's or a path's placement from the mesh itself while
/// the two are read against the very same body.
pub(crate) const REFERENCE: toile_anny::phenotype::Phenotype = toile_anny::phenotype::Phenotype {
    gender: 0.0,
    age: 0.8,
    muscle: 0.5,
    weight: 0.5,
    height: 0.5,
    proportions: 0.5,
};

/// Loads the reference adult Anny body ([`REFERENCE`]) and hashes the bits
/// of its mesh.
pub fn anny_mesh_hash() -> u64 {
    let mesh = toile_anny::body_mesh(&REFERENCE, &[0.0; 20]);
    let mut h = FNV_BASIS;
    for f in mesh.positions.iter().chain(&mesh.normals) {
        h = (h ^ u64::from(f.to_bits())).wrapping_mul(FNV_PRIME);
    }
    for &i in &mesh.indices {
        h = (h ^ u64::from(i)).wrapping_mul(FNV_PRIME);
    }
    h
}

/// Loads the same reference adult Anny body [`anny_mesh_hash`] hashes and
/// hashes its 20 catalogue measurements together.
///
/// Reads them with `crate::body::measured_anny`, in the catalogue's own
/// fixed order (`toile_doc::MeasureSet::CATALOGUE`). Only `+ - * / sqrt`
/// reaches a measured number, so this is bit-identical on macOS ARM and
/// Linux x86 the same way the others are.
///
/// # Panics
/// If `measured_anny` ever omits one of the catalogue's own 20 names: that
/// would mean the two have drifted apart, which is a bug to fix rather
/// than a shape to hash around.
pub fn anny_measures_hash() -> u64 {
    let mesh = crate::body::body_mesh(&REFERENCE, &crate::body::NO_LEVERS);
    let measured = crate::body::measured_anny(&mesh);
    let mut h = FNV_BASIS;
    for name in toile_doc::MeasureSet::CATALOGUE {
        let v = measured
            .get(name)
            .unwrap_or_else(|| panic!("measured_anny did not report `{name}`"));
        h = (h ^ v.to_bits()).wrapping_mul(FNV_PRIME);
    }
    h
}

/// Solves every lever against Toile's own default tape and hashes the
/// result.
///
/// Runs against `crate::body::default_measures` for a fixed,
/// gender-unspecified adult phenotype, and hashes the 20 solved lever
/// values together with the resulting mesh's positions and normals.
///
/// # Panics
/// If the shipped asset fails to decode: see `toile_anny::body_mesh`'s doc.
pub fn anny_solved_hash() -> u64 {
    let phenotype = toile_anny::phenotype::Phenotype::default();
    let measures = crate::body::default_measures();
    let solved = crate::body::solve_anny(&measures, &phenotype);
    let mesh = toile_anny::body_mesh(&solved.phenotype, &solved.levers);

    let mut h = FNV_BASIS;
    for &t in &solved.levers {
        h = (h ^ t.to_bits()).wrapping_mul(FNV_PRIME);
    }
    for f in mesh.positions.iter().chain(&mesh.normals) {
        h = (h ^ u64::from(f.to_bits())).wrapping_mul(FNV_PRIME);
    }
    for &i in &mesh.indices {
        h = (h ^ u64::from(i)).wrapping_mul(FNV_PRIME);
    }
    h
}

/// Bakes the reference adult Anny body ([`REFERENCE`]) into its collision
/// field and hashes the whole grid: the counts, where it sits, and every
/// voxel.
///
/// It moves when the body moves, and when anything in the bake moves: the
/// cell, the band, the pseudo-normals, the case analysis, the flood fill, or
/// the `libm` the angles come from (see `body::bake::BAKE_VERSION`).
///
/// # Panics
/// If the Anny body stops being a closed, orientable 2-manifold. That is not
/// a shape to hash around: a field baked from a mesh with a hole has a sign
/// that is wrong somewhere, and the bake refuses it.
pub fn anny_sdf_hash() -> u64 {
    let mesh = toile_anny::body_mesh(&REFERENCE, &[0.0; 20]);
    let sdf = crate::body::bake::sdf(&mesh).expect("the Anny body is closed and orientable");
    field_hash(&sdf)
}

/// Hashes a whole baked field: the counts, where it sits, and every voxel.
pub(crate) fn field_hash(sdf: &SdfGrid) -> u64 {
    let mut h = FNV_BASIS;
    for n in sdf.dims {
        h = (h ^ n as u64).wrapping_mul(FNV_PRIME);
    }
    for f in sdf
        .data
        .iter()
        .chain(&sdf.origin)
        .chain(std::iter::once(&sdf.cell))
    {
        h = (h ^ u64::from(f.to_bits())).wrapping_mul(FNV_PRIME);
    }
    h
}

/// Bakes the demo's analytic sphere field and hashes its bits, with the cell
/// size and origin that place it.
///
/// The drape golden already covers this field through 1,200 substeps, but it
/// only sees the handful of voxels the cloth touches. This sees all of them,
/// so a change to the grid that the bodice would never have fallen on is
/// caught where it happens rather than never.
pub fn sphere_sdf_hash() -> u64 {
    let sdf = demo::avatar_sdf();
    let mut h = FNV_BASIS;
    for f in sdf
        .data
        .iter()
        .chain(&sdf.origin)
        .chain(std::iter::once(&sdf.cell))
    {
        h = (h ^ u64::from(f.to_bits())).wrapping_mul(FNV_PRIME);
    }
    h
}

/// Drapes the demo bodice over the reference adult Anny body and hashes the
/// result.
///
/// The same panel the sphere golden drops, let go at the height this body
/// decides rather than at the sphere's, over the field [`anny_sdf_hash`]
/// pins. Counted in substeps against the scalar reference solver: no wall
/// clock and no threads, so the answer is a property of the code rather than
/// of the machine.
///
/// It moves when the body moves, when the bake moves, and when the release
/// height stops being derived the way it is. The sphere's own drape golden is
/// a separate scene and does not move with any of them.
///
/// # Panics
/// If the Anny body stops being closed and orientable, or the demo contour
/// stops being one the mesher accepts.
pub fn drape_on_anny_hash() -> u64 {
    const DT: f32 = 1.0 / 600.0;
    const SUBSTEPS: usize = 1200;

    let mesh = toile_anny::body_mesh(&REFERENCE, &[0.0; 20]);
    let body = crate::body::Collider::bake(&mesh).expect("the Anny body is closed and orientable");
    let no_seams = Seams::default();
    let contour = demo::bodice_contour();
    let pipe = demo::pipeline(&contour);
    let mut state = crate::couture::drop_state(&pipe, body.release_height());
    let cons = pipe.constraints(crate::couture::COMPLIANCE);

    // No floor, though this body stands on one, and the pull the solver has
    // always had. A golden pins the arithmetic the solver has always run; the
    // ground belongs to the scene a person is looking at, not to the one the
    // bits are taken in.
    let stage = Stage::around(body.field());
    for _ in 0..SUBSTEPS {
        xpbd::substep(&mut state, &cons, &no_seams, &stage, None, DT);
    }
    xpbd::position_hash(&state)
}

/// Drapes the demo bodice, moves the shoulder point 2 cm, drapes again, and
/// hashes the result.
///
/// Counted in substeps against the scalar reference solver: no wall clock, no
/// threads, so the answer is a property of the code rather than of the machine.
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
    let stage = Stage::around(&sdf);

    for _ in 0..SUBSTEPS {
        xpbd::substep(&mut state, &cons, &no_seams, &stage, None, DT);
    }
    contour[demo::SHOULDER_POINT][0] += 0.02;
    let rests = pipe
        .derive(&contour)
        .expect("the golden moves a point, never the node count");
    cons.rest.copy_from_slice(rests);
    for _ in 0..SUBSTEPS {
        xpbd::substep(&mut state, &cons, &no_seams, &stage, None, DT);
    }
    xpbd::position_hash(&state)
}
