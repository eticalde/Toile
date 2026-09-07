use toile_sim::xpbd::State;

use super::{COMPLIANCE, DT, PieceSlot, SUBSTEPS_PER_TICK, SessionError};
use crate::couture::{self, ShapePipeline};
use crate::demo;
use crate::draft::{Draft, PieceKey};
use crate::sync::{self, SimHandle};

/// Meshes a piece and seeds its drape: the slot, its contour, and the dropped
/// starting state, shared by opening a document and adopting a drawn piece.
///
/// # Errors
/// `SessionError` when the piece carries a defect or a contour the mesher
/// refuses.
pub(super) fn drape_piece(
    draft: &Draft,
    piece: PieceKey,
) -> Result<(PieceSlot, Vec<[f64; 2]>, State), SessionError> {
    if let [defect, ..] = draft.defects(piece) {
        return Err(SessionError::Defective {
            piece,
            defect: defect.clone(),
        });
    }
    let contour = draft.outline(piece).to_vec();
    let (samples, max_area) = couture::for_contour(&contour);
    let pipeline = ShapePipeline::build(&contour, samples, max_area)?;
    let state = couture::drop_state(&pipeline, couture::DROP_HEIGHT);
    let slot = PieceSlot::new(pipeline, draft.topology(piece));
    Ok((slot, contour, state))
}

/// Starts the sim thread around a meshed piece.
pub(super) fn spawn_sim(slot: &PieceSlot, state: State) -> SimHandle {
    let cons = slot.pipeline().constraints(COMPLIANCE);
    sync::spawn(
        state,
        cons,
        demo::avatar_sdf(),
        slot.pipeline().tris.clone(),
        DT,
        SUBSTEPS_PER_TICK,
    )
}
