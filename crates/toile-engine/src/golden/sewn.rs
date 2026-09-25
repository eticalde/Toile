use toile_sim::xpbd::{self, Floor, Seams, Stage};

use super::REFERENCE;
use crate::couture::{self, COMPLIANCE, SEAM_PASSES, SEAM_STEP, ShapePipeline};
use crate::draft::{Draft, block};
use crate::session::pair_seam_anchored;

/// Substeps the sewn drape is counted for.
///
/// Ten simulated seconds, where the other two goldens stop at one and two: far
/// past the ramp the sewing goes firm at, and far enough that the garment is
/// lying on the body rather than still on its way down. What a sewn product
/// does after its seams shut is the half of it nothing pinned.
const SUBSTEPS: u64 = 6000;

/// Simulated seconds per substep, the reference solver's own.
const DT: f32 = 1.0 / 600.0;

/// Drapes the shipped block's two pieces sewn together, over the reference
/// adult Anny body, and hashes where they come to.
///
/// The gap the other two drape goldens leave: both run `Seams::default()`, so
/// `solve_seams` never executes in either, and until this one nothing pinned a
/// garment that is sewn at all. What moves it and moves neither of them is the
/// pairing, the sewing schedule, the weightless phase, and each of those
/// against the body. What it does not watch is on its own test.
///
/// The sim thread's own schedule, counted in substeps rather than clocked, and
/// not its kinetic damper: that belongs to how a drape is brought to rest
/// rather than to the arithmetic a golden pins. The ground is here where the
/// other two refuse it, because a garment with nothing under it never rests.
///
/// # Panics
/// If the shipped block stops resolving, stops meshing or stops pairing its own
/// seams, or if the Anny body stops being closed and orientable.
pub fn drape_sewn_hash() -> u64 {
    let mesh = toile_anny::body_mesh(&REFERENCE, &[0.0; 20]);
    let body = crate::body::Collider::bake(&mesh).expect("the Anny body is closed and orientable");
    let draft = Draft::from_doc(block::trousers()).expect("the block resolves");
    let pipes = meshed(&draft);
    let refs: Vec<&ShapePipeline> = pipes.iter().collect();
    let mut seams = sewn(&draft, &refs);
    let cons = couture::combine_constraints(&refs, COMPLIANCE);
    let mut state = couture::drop_all(&refs, body.release_height(), None);

    let stage = Stage::around(body.field()).on(body.ground().map_or(Floor::none(), Floor::at));
    for substep in 0..SUBSTEPS {
        (seams.compliance, seams.max_step) = couture::sewing_at(substep);
        let gap = xpbd::seam_gap(&state, &seams);
        let stage = if couture::closing(substep, gap) {
            stage.weightless()
        } else {
            stage
        };
        xpbd::substep(&mut state, &cons, &seams, &stage, None, DT);
    }
    xpbd::position_hash(&state)
}

/// The block's front and back meshed, in that order.
///
/// Named rather than taken from the document's own order, so that a piece added
/// to the block one day is a new golden's business and not a silent change to
/// this one's scene.
fn meshed(draft: &Draft) -> Vec<ShapePipeline> {
    [block::FRONT, block::BACK]
        .iter()
        .map(|name| {
            let piece = draft
                .doc()
                .piece_named(name)
                .expect("the block draws a front and a back");
            let contour = draft.outline_m(piece);
            let (samples, area) = couture::for_contour(contour);
            ShapePipeline::build(contour, samples, area).expect("the block's pieces mesh")
        })
        .collect()
}

/// Every seam of the document paired onto the combined state, at the softness
/// a drape begins with.
///
/// Through [`pair_seam_anchored`], which is the rule the studio pairs by, so
/// this scene is sewn the way a person's product is and not by a second reading
/// of the same document.
fn sewn(draft: &Draft, pipes: &[&ShapePipeline]) -> Seams {
    let bases = couture::offsets(pipes);
    let named = [block::FRONT, block::BACK].map(|name| draft.doc().piece_named(name));
    let mut seams = Seams {
        compliance: 0.0,
        max_step: SEAM_STEP,
        iterations: SEAM_PASSES,
        ..Seams::default()
    };
    for (_, seam) in draft.doc().seams.iter() {
        let side = |at: Option<crate::draft::PieceKey>| {
            at.and_then(|key| named.iter().position(|&k| k == Some(key)))
        };
        let (Some(ia), Some(ib)) = (side(seam.a.piece()), side(seam.b.piece())) else {
            continue;
        };
        let (a, b) = pair_seam_anchored(draft, seam, pipes[ia], bases[ia], pipes[ib], bases[ib])
            .expect("the block's own seams pair");
        seams.a.extend(a);
        seams.b.extend(b);
    }
    seams
}
