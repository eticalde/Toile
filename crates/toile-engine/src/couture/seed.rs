use toile_sim::xpbd::State;

use super::pipeline::ShapePipeline;

/// Height the cloth is released from, in metres.
pub const DROP_HEIGHT: f32 = 0.35;

/// Seeds the solver: the piece centred over the avatar and released flat.
///
/// The piece is centred on its own vertices rather than on its bounding box,
/// so a contour that grows on one side does not swing the whole panel across
/// the avatar before it has fallen.
pub fn drop_state(pipeline: &ShapePipeline, height: f32) -> State {
    let mut state = State::new(pipeline.pos2d.len());
    release_into(&mut state, 0, pipeline, height);
    state
}

/// Writes one piece's released block into a state that may hold others,
/// beginning at `base`.
///
/// The pieces of a product are concatenated, so the only thing between seeding
/// one piece and seeding the whole of [`drop_state`] is where its block
/// begins. The first piece begins at zero and nothing is added to anything,
/// which is what makes a product of one piece exactly the arithmetic this was
/// written as.
pub(super) fn release_into(state: &mut State, base: usize, pipeline: &ShapePipeline, height: f32) {
    let n = pipeline.pos2d.len();
    let (mut cx, mut cy) = (0.0, 0.0);
    for p in &pipeline.pos2d {
        cx += p[0];
        cy += p[1];
    }
    cx /= n as f64;
    cy /= n as f64;

    for i in 0..n {
        state.px[base + i] = (pipeline.pos2d[i][0] - cx) as f32;
        state.py[base + i] = height;
        state.pz[base + i] = (pipeline.pos2d[i][1] - cy) as f32;
    }
}
