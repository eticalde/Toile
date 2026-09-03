use toile_sim::xpbd::State;

use super::pipeline::ShapePipeline;

/// Carries the live drape onto a re-meshed piece.
///
/// Each vertex of the new mesh is located in the old mesh's 2D rest space and
/// its position and velocity interpolated barycentrically, so a topology
/// change continues the drape instead of restarting it.
pub fn transfer_state(old: &ShapePipeline, old_state: &State, new: &ShapePipeline) -> State {
    let loc = toile_mesh::transfer::Locator::build(&old.pos2d, &old.tris);
    let mut s = State::new(new.pos2d.len());
    for (i, p) in new.pos2d.iter().enumerate() {
        let (t, b) = loc.locate(*p);
        let (ia, ib, ic) = (
            old.tris[t * 3] as usize,
            old.tris[t * 3 + 1] as usize,
            old.tris[t * 3 + 2] as usize,
        );
        let lerp = |src: &[f32], out: &mut [f32]| {
            out[i] = (b[0] * f64::from(src[ia])
                + b[1] * f64::from(src[ib])
                + b[2] * f64::from(src[ic])) as f32;
        };
        lerp(&old_state.px, &mut s.px);
        lerp(&old_state.py, &mut s.py);
        lerp(&old_state.pz, &mut s.pz);
        lerp(&old_state.vx, &mut s.vx);
        lerp(&old_state.vy, &mut s.vy);
        lerp(&old_state.vz, &mut s.vz);
    }
    s
}
