use toile_engine::body::{BodyMesh, Collider, bake};
use toile_engine::couture::{HOLDS_ITS_RATIO, SEAM_SHUT, ShapePipeline, for_contour};
use toile_engine::session::Session;
use toile_sim::xpbd::SdfGrid;

use crate::extremes::GATHERED;
use crate::skirt::{Cut, cut_to};

/// A published frame's flat triple-per-vertex positions, as points.
pub fn placed(flat: &[f32]) -> Vec<[f32; 3]> {
    flat.as_chunks::<3>().0.to_vec()
}

/// How far apart two placed vertices stand, in metres.
pub fn apart(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = [0, 1, 2].map(|k| b[k] - a[k]);
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// How far the two sides of a garment's seams stand apart, in metres: the
/// worst sewn pair and the mean over all of them.
///
/// `None` for a garment with no seams, which is a different answer from zero:
/// a single panel has nothing to close and nothing to say about fit.
pub fn gaps(pairs: &[(u32, u32)], at: &[[f32; 3]]) -> Option<(f32, f32)> {
    let (mut worst, mut sum) = (0.0f32, 0.0f32);
    for &(a, b) in pairs {
        let d = apart(at[a as usize], at[b as usize]);
        worst = worst.max(d);
        sum += d;
    }
    (!pairs.is_empty()).then(|| (worst, sum / pairs.len() as f32))
}

/// Prints that reading and hands it back.
///
/// In millimetres and not in metres, which is the whole of why this is one
/// call and not a `println!` per scene: read in metres a shut seam and a seam
/// a millimetre open both print `0.000`, and the number this suite exists to
/// show is 115.3 mm.
pub fn report(
    scene: &str,
    when: &str,
    pairs: &[(u32, u32)],
    at: &[[f32; 3]],
) -> Option<(f32, f32)> {
    let read = gaps(pairs, at);
    match read {
        Some((worst, mean)) => println!(
            "{scene} seams {when}: {} pairs, worst {:.1} mm, mean {:.1} mm",
            pairs.len(),
            worst * 1000.0,
            mean * 1000.0
        ),
        None => println!("{scene} seams {when}: none — a single panel closes nothing"),
    }
    read
}

/// Reads a scene's seams and holds them to shut.
///
/// [`SEAM_SHUT`] and not a number of this suite's own. What an open seam at
/// rest is worth nobody has decided, and a threshold invented here would be
/// worse than the printed number it replaced — but *shut* is decided, and
/// decided by measurement: it is the distance inside which the solver's own
/// closing phase stops waiting for a seam. Asking the same question the
/// sewing asks is the only form of this assertion that cannot drift from it.
///
/// The worst pair and not the mean. A mean over 353 pairs reads 3.4 mm while
/// one side seam stands 115.3 mm open, so a garment that does not fit passes
/// on the mean; what says a seam met is that every pair of it did.
///
/// # Panics
/// If the scene has no seams, or any pair stands further apart than that.
pub fn shut(scene: &str, when: &str, pairs: &[(u32, u32)], at: &[[f32; 3]]) {
    let Some((worst, _)) = report(scene, when, pairs, at) else {
        panic!("{scene}: held to its seams {when}, and it has none");
    };
    assert!(
        worst <= SEAM_SHUT,
        "{scene}: the seams did not shut {when}: worst pair {:.1} mm apart, \
         against the {:.1} mm the closing phase counts as shut",
        worst * 1000.0,
        SEAM_SHUT * 1000.0
    );
}

/// The pieces of a session meshed again, in the order it holds them.
///
/// The engine's own pipelines are not a test's to see, so this builds the same
/// ones the same way: the same contours, through the same density rule. What it
/// buys is the pattern a vertex came from, which is what any reading of where
/// the placement put it has to be taken against.
///
/// # Panics
/// If the scene was not opened from a document, or the meshes come out a
/// different size from the ones the session is standing on.
pub fn meshed(session: &Session) -> Vec<ShapePipeline> {
    let draft = session
        .draft()
        .expect("the scene was opened from a document");
    let pipes: Vec<ShapePipeline> = session
        .pieces()
        .iter()
        .map(|&key| {
            let contour = draft.outline_m(key);
            let (samples, area) = for_contour(contour);
            ShapePipeline::build(contour, samples, area).expect("the pieces mesh")
        })
        .collect();
    assert_eq!(
        pipes.iter().map(|pipe| pipe.pos2d.len()).sum::<usize>(),
        session.n_vertices(),
        "the meshes a test builds are the meshes the session is standing on"
    );
    pipes
}

/// Bakes a body and lets the skirt drafted to it go, without draping it.
pub fn let_go(mesh: &BodyMesh, cut: Cut) -> (Session, SdfGrid) {
    let body = Collider::bake(mesh).expect("the Anny body is closed and orientable");
    let sdf = bake::sdf(mesh).expect("the Anny body is closed and orientable");
    let doc = cut_to(cut, Some((GATHERED, HOLDS_ITS_RATIO)));
    (Session::from_doc(doc, body).expect("the skirt drapes"), sdf)
}

/// What rolling costs the cloth: every edge against its own flat length.
mod stretch;
/// The surface a garment is let go on, and every line of cloth's place on it.
mod surface;
