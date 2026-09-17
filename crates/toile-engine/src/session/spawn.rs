use toile_sim::xpbd::State;

use super::{COMPLIANCE, DT, PieceSlot, SUBSTEPS_PER_TICK, SessionError};
use crate::body::Collider;
use crate::couture::{self, ShapePipeline};
use crate::draft::{Draft, PieceKey};
use crate::sync::{self, SimHandle};

/// Meshes a piece and seeds its drape: the slot, its contour, and the dropped
/// starting state, shared by opening a document and adopting a drawn piece.
///
/// `release` is the height the piece is let go from, which the body it will
/// fall on decides: a garment starts over the body it is being fitted to, and
/// a person is not a ball.
///
/// # Errors
/// `SessionError` when the piece carries a defect or a contour the mesher
/// refuses.
pub(super) fn drape_piece(
    draft: &Draft,
    piece: PieceKey,
    release: f32,
) -> Result<(PieceSlot, Vec<[f64; 2]>, State), SessionError> {
    if let [defect, ..] = draft.defects(piece) {
        return Err(SessionError::Defective {
            piece,
            defect: defect.clone(),
        });
    }
    let contour = draft.outline_m(piece).to_vec();
    let (samples, max_area) = couture::for_contour(&contour);
    let pipeline = ShapePipeline::build(&contour, samples, max_area)?;
    let state = couture::drop_state(&pipeline, release);
    let slot = PieceSlot::new(pipeline, draft.topology(piece));
    Ok((slot, contour, state))
}

/// Starts the sim thread around a meshed piece, colliding against `collider`.
///
/// The field crosses as a second owner of the same voxels, so starting a
/// thread over a body already baked copies a pointer rather than tens of
/// megabytes.
pub(super) fn spawn_sim(slot: &PieceSlot, state: State, collider: &Collider) -> SimHandle {
    let cons = slot.pipeline().constraints(COMPLIANCE);
    sync::spawn(
        state,
        cons,
        collider.shared(),
        slot.pipeline().tris.clone(),
        DT,
        SUBSTEPS_PER_TICK,
    )
}
