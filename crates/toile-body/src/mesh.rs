use crate::landmarks::landmarks;
use crate::loft::vertex_normals;
use crate::params::{BodyMeasures, BodyRes};
use crate::parts::{Ctx, Part, Side, arm, leg, trunk};
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

/// The five parts in their fixed order, uncentred (ankle joint at y = 0):
/// trunk with neck and head, left leg, right leg, left arm, right arm.
pub(crate) fn parts(m: &BodyMeasures, res: BodyRes) -> [Part; 5] {
    let dirs = unit_dirs(res.seg);
    let lm = landmarks(m);
    let c = Ctx {
        m,
        res,
        lm: &lm,
        dirs: &dirs,
    };
    [
        trunk(&c),
        leg(&c, Side::Left),
        leg(&c, Side::Right),
        arm(&c, Side::Left),
        arm(&c, Side::Right),
    ]
}

/// Lofts the body for one set of measurements at the given resolution.
///
/// The parts are concatenated with their indices rebased by the running
/// vertex count — the same rebase the sphere's index plan uses — then the
/// figure is centred vertically and its normals are computed from the final
/// positions.
pub fn body_mesh(m: &BodyMeasures, res: BodyRes) -> BodyMesh {
    let mut positions: Vec<f32> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    for (verts, idx) in parts(m, res) {
        let base = (positions.len() / 3) as u32;
        positions.extend_from_slice(&verts);
        indices.extend(idx.into_iter().map(|i| i + base));
    }

    // Centre the figure on the origin: the parts are built with the ankle joint
    // at y = 0 and the crown at y = crown, so shift down by half the height.
    let shift = (landmarks(m).crown * 0.5) as f32;
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
