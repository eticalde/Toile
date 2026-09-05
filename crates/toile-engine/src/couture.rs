mod density;
mod pipeline;
mod seam;
mod seed;
mod transfer;

pub use density::for_contour;
pub use pipeline::{COMPLIANCE, RestStateError, ShapePipeline};
pub use seam::pair_seam;
pub use seed::{DROP_HEIGHT, drop_state};
pub use transfer::{MeshSwap, onto, transfer_state};
