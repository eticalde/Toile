use toile_sim::xpbd::{DistanceConstraints, Floor, Seams};

use super::{DT, PieceSlot, SUBSTEPS_PER_TICK, SessionError};
use crate::body::Collider;
use crate::couture::{self, Layout, ShapePipeline};
use crate::draft::{Draft, PieceKey};
use crate::sync::{self, Scene, SimHandle};

/// What a body offers the solver: its field, and the plane it stands on.
///
/// The one place the two are put together, so a body handed to a running sim
/// and a body a fresh thread starts on cannot come to disagree about where
/// the ground is.
pub(super) fn scene_of(collider: &Collider) -> Scene {
    Scene {
        sdf: collider.shared(),
        floor: collider.ground().map_or(Floor::none(), Floor::at),
    }
}

/// Meshes a piece: the slot it goes in, and the contour it was meshed from.
///
/// Where it is let go is not decided here. A piece falls with the rest of the
/// product, from a height the body decides and at a place its seams decide,
/// and one piece of a product has no starting state of its own — see
/// [`spawn_sim`].
///
/// # Errors
/// `SessionError` when the piece carries a defect or a contour the mesher
/// refuses.
pub(super) fn drape_piece(
    draft: &Draft,
    piece: PieceKey,
) -> Result<(PieceSlot, Vec<[f64; 2]>), SessionError> {
    if let [defect, ..] = draft.defects(piece) {
        return Err(SessionError::Defective {
            piece,
            defect: defect.clone(),
        });
    }
    let contour = draft.outline_m(piece).to_vec();
    let (samples, max_area) = couture::for_contour(&contour);
    let pipeline = ShapePipeline::build(&contour, samples, max_area)?;
    let slot = PieceSlot::new(pipeline, draft.topology(piece));
    Ok((slot, contour))
}

/// Starts the sim thread around every piece of the product, colliding against
/// `collider`, held to `cons` and sewn along `seams`.
///
/// One state and one thread whatever the product holds, because a seam joins
/// two vertices *within* a state: pieces sewn to each other have to be solved
/// together or they cannot be sewn at all. The field crosses as a second owner
/// of the same voxels, so starting a thread over a body already baked copies a
/// pointer rather than tens of megabytes.
///
/// `around` is the ring the seams and the body put the pieces on, and it
/// carries its own height. A piece it does not place — a lone panel, which has
/// no partner to be placed against — is let go exactly as a lone piece is let
/// go today, flat at the height the body decides.
///
/// The constraints arrive rather than being compiled here: what an edge rests
/// at is the session's answer and not the mesh's, because an elastic is
/// written over a stretch only the table holding the document can place.
pub(super) fn spawn_sim(
    pipelines: &[&ShapePipeline],
    tris: Vec<u32>,
    cons: DistanceConstraints,
    seams: Seams,
    around: Option<&Layout>,
    collider: &Collider,
) -> SimHandle {
    let state = couture::drop_all(pipelines, collider.release_height(), around);
    sync::spawn(
        state,
        cons,
        seams,
        scene_of(collider),
        tris,
        DT,
        SUBSTEPS_PER_TICK,
    )
}
