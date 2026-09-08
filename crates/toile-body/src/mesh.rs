use crate::loft::{hip_half_width, landmarks, leg, trunk, vertex_normals};
use crate::params::{BodyMeasures, BodyRes};
use crate::ring::unit_dirs;

/// A triangulated body surface in the renderer's terms.
///
/// Metres, y-up, centred on the origin, CCW-outward winding, unit outward
/// normals. The fields mirror `toile_engine::sync::Snapshot`, so the app's
/// existing interleave loop transfers verbatim.
pub struct BodyMesh {
    /// Vertex positions as xyz triples.
    pub positions: Vec<f32>,
    /// Unit outward normals, one xyz triple per position.
    pub normals: Vec<f32>,
    /// CCW triangle indices into `positions`.
    pub indices: Vec<u32>,
}

impl BodyMesh {
    /// The number of vertices in the mesh.
    pub fn vertex_count(&self) -> usize {
        self.positions.len() / 3
    }
}

/// Lofts the body for one set of measurements at the given resolution.
///
/// The three parts (trunk-with-dome and two legs) are concatenated with their
/// indices rebased by the running vertex count — the same rebase the sphere's
/// index plan uses — then the figure is centred vertically and its normals are
/// computed from the final positions.
pub fn body_mesh(m: &BodyMeasures, res: BodyRes) -> BodyMesh {
    let dirs = unit_dirs(res.seg);
    let lm = landmarks(m);
    let dx = hip_half_width(m, &dirs);

    let mut positions: Vec<f32> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for (verts, idx) in [
        trunk(m, res, &lm, &dirs),
        leg(m, res, &lm, -dx, &dirs),
        leg(m, res, &lm, dx, &dirs),
    ] {
        let base = (positions.len() / 3) as u32;
        positions.extend_from_slice(&verts);
        indices.extend(idx.into_iter().map(|i| i + base));
    }

    // Centre the figure on the origin: the parts are built with the ankle joint
    // at y = 0 and the crown at y = crown, so shift down by half the height.
    let shift = (lm.crown * 0.5) as f32;
    for y in positions.iter_mut().skip(1).step_by(3) {
        *y -= shift;
    }

    let normals = vertex_normals(&positions, &indices);
    BodyMesh {
        positions,
        normals,
        indices,
    }
}
