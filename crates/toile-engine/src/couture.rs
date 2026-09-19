mod density;
mod elastic;
mod pipeline;
mod place;
mod product;
mod seam;
mod seed;
mod transfer;

pub use density::for_contour;
pub use elastic::{HOLDS_ITS_RATIO, Held, compliance_of, hold};
pub use pipeline::{COMPLIANCE, RestStateError, ShapePipeline};
pub use place::{Layout, Wrap};
pub use product::{combine_constraints, combine_triangles, drop_all, offsets};
pub use seam::{SEAM_PASSES, SEAM_STEP, closing, pair_seam, sewing_at};
pub use seed::{DROP_HEIGHT, drop_state};
pub use transfer::{MeshSwap, onto, transfer_state};
