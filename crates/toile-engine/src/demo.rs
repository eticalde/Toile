use toile_sim::xpbd::{SdfGrid, State};

use crate::couture::{self, ShapePipeline};

/// Boundary samples taken around the demo contour.
const SAMPLES: usize = 256;

/// Triangle area cap for the demo mesh: roughly 6 mm sides.
const MAX_AREA: f64 = 2.0e-5;

/// Radius of the sphere standing in for the avatar, in metres.
pub const AVATAR_RADIUS: f32 = 0.15;

/// A front bodice panel with a concave armhole and neckline.
///
/// Shared by the goldens, the benchmarks and the desktop demo: defining the
/// scene once is what stops the five of them from drifting apart.
pub fn bodice_contour() -> Vec<[f64; 2]> {
    let mut pts: Vec<[f64; 2]> = Vec::new();
    let line = |pts: &mut Vec<[f64; 2]>, a: [f64; 2], b: [f64; 2], n: usize| {
        for i in 0..n {
            let t = i as f64 / n as f64;
            pts.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
        }
    };
    let quad = |pts: &mut Vec<[f64; 2]>, a: [f64; 2], c: [f64; 2], b: [f64; 2], n: usize| {
        for i in 0..n {
            let t = i as f64 / n as f64;
            let u = 1.0 - t;
            pts.push([
                u * u * a[0] + 2.0 * u * t * c[0] + t * t * b[0],
                u * u * a[1] + 2.0 * u * t * c[1] + t * t * b[1],
            ]);
        }
    };
    line(&mut pts, [0.0, 0.0], [0.50, 0.0], 20); // waist
    line(&mut pts, [0.50, 0.0], [0.52, 0.45], 18); // side seam
    quad(&mut pts, [0.52, 0.45], [0.36, 0.50], [0.38, 0.68], 30); // armhole
    line(&mut pts, [0.38, 0.68], [0.18, 0.72], 10); // shoulder
    quad(&mut pts, [0.18, 0.72], [0.16, 0.56], [0.0, 0.60], 26); // neckline
    line(&mut pts, [0.0, 0.60], [0.0, 0.0], 24); // centre front
    pts
}

/// Index of the shoulder-to-armhole point, the one the goldens and the drag
/// storms move.
pub const SHOULDER_POINT: usize = 68;

/// Meshes a contour at the demo's sampling density.
///
/// The two constants are the scene the goldens hash, so this stays a literal
/// density rather than one derived from the contour.
///
/// # Panics
/// If the contour is not finite, which this module's own contour is.
pub fn pipeline(contour: &[[f64; 2]]) -> ShapePipeline {
    ShapePipeline::build(contour, SAMPLES, MAX_AREA).expect("the demo contour is finite")
}

/// Seeds the solver: the piece centred over the avatar and released flat.
pub fn drop_state(pipeline: &ShapePipeline) -> State {
    couture::drop_state(pipeline, couture::DROP_HEIGHT)
}

/// The avatar's collision field.
pub fn avatar_sdf() -> SdfGrid {
    SdfGrid::sphere(
        256,
        1.4 / 255.0,
        [-0.7, -0.7, -0.7],
        [0.0, 0.0, 0.0],
        AVATAR_RADIUS,
    )
}

#[cfg(test)]
mod tests {
    use toile_sim::xpbd;

    use super::*;

    /// Every bit the seeding produced before it moved out of this module.
    ///
    /// The drape golden starts from this state, so a rounding difference here
    /// is a different drape. A cheap rectangle stands in for the bodice: what
    /// is under test is the arithmetic, not the scene.
    #[test]
    fn drop_state_is_unchanged_by_the_move_into_couture() {
        let rectangle = [[0.0, 0.0], [0.30, 0.0], [0.30, 0.20], [0.0, 0.20]];
        let pipe = ShapePipeline::build(&rectangle, 16, 0.01).expect("the rectangle is finite");
        let state = drop_state(&pipe);
        assert_eq!(state.px.len(), 21);
        assert_eq!(xpbd::position_hash(&state), 0xc30d_7f61_e15f_378c);
    }
}
