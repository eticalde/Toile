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
    let n = pipeline.pos2d.len();
    let (mut cx, mut cy) = (0.0, 0.0);
    for p in &pipeline.pos2d {
        cx += p[0];
        cy += p[1];
    }
    cx /= n as f64;
    cy /= n as f64;

    let mut state = State::new(n);
    for i in 0..n {
        state.px[i] = (pipeline.pos2d[i][0] - cx) as f32;
        state.py[i] = height;
        state.pz[i] = (pipeline.pos2d[i][1] - cy) as f32;
    }
    state
}
