/// Half the ground's side, in metres.
///
/// The Anny body reaches 0.61 m to a fingertip, so three metres across leaves
/// a person standing well inside it at the framing the viewport opens on.
const REACH: f32 = 1.5;

/// Spacing between the lines, in metres: a decimetre, the unit a pattern is
/// read in, so the grid doubles as a rule against the garment over it.
const STEP: f32 = 0.1;

/// Half a drawn line's width, in metres.
const HALF_WIDTH: f32 = 0.0015;

/// How far under the plane the grid is drawn, in metres.
///
/// The solver rests cloth exactly on the plane, so a grid drawn at the same
/// height would fight the hem pooling on it for every pixel of the depth
/// buffer — the quarrel the avatar settles by drawing just inside its own
/// skin. Two millimetres is under the 5 mm the field is sampled at, so the
/// ground still reads as the surface the body stands on.
const UNDERFOOT: f32 = 0.002;

/// The ground as a grid of thin quads: interleaved position, normal and
/// colour, and the triangles over them.
///
/// Drawn, never collided against — the floor the cloth actually stops at is
/// the solver's plane, and this is the picture of it.
pub fn grid(y: f32, centre: [f32; 2], color: [f32; 3]) -> (Vec<f32>, Vec<u32>) {
    let at = y - UNDERFOOT;
    let lines = (REACH / STEP) as i32;
    let mut verts: Vec<f32> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    let mut quad = |corners: [[f32; 3]; 4]| {
        let base = (verts.len() / 9) as u32;
        for p in corners {
            verts.extend_from_slice(&p);
            verts.extend_from_slice(&[0.0, 1.0, 0.0]);
            verts.extend_from_slice(&color);
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    };
    for k in -lines..=lines {
        let step = k as f32 * STEP;
        let x = centre[0] + step;
        quad([
            [x - HALF_WIDTH, at, centre[1] - REACH],
            [x + HALF_WIDTH, at, centre[1] - REACH],
            [x + HALF_WIDTH, at, centre[1] + REACH],
            [x - HALF_WIDTH, at, centre[1] + REACH],
        ]);
        let z = centre[1] + step;
        quad([
            [centre[0] - REACH, at, z - HALF_WIDTH],
            [centre[0] - REACH, at, z + HALF_WIDTH],
            [centre[0] + REACH, at, z + HALF_WIDTH],
            [centre[0] + REACH, at, z - HALF_WIDTH],
        ]);
    }
    (verts, idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two runs of lines, each a quad of four vertices and two triangles.
    #[test]
    fn the_grid_rules_both_ways_across_the_plane() {
        let (verts, idx) = grid(-0.837, [0.0, 0.0], [0.1, 0.1, 0.1]);
        let quads = 2 * (2 * (REACH / STEP) as usize + 1);
        assert_eq!(verts.len(), quads * 4 * 9);
        assert_eq!(idx.len(), quads * 6);
        assert!(
            idx.iter().all(|&i| (i as usize) < verts.len() / 9),
            "every triangle indexes a vertex the grid actually carries"
        );
    }

    /// It is drawn under the plane the cloth rests on, never level with it:
    /// level, the hem pooling on the floor would z-fight the floor.
    #[test]
    fn the_grid_sits_just_under_the_plane_it_draws() {
        let plane = -0.837;
        let (verts, _) = grid(plane, [0.0, 0.0], [0.1, 0.1, 0.1]);
        for vertex in verts.as_chunks::<9>().0 {
            assert!(vertex[1] < plane, "at {} against {plane}", vertex[1]);
            assert_eq!(vertex[3..6], [0.0, 1.0, 0.0], "a plane faces straight up");
        }
    }
}
