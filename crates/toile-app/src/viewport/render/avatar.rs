use toile_engine::draft::BodyMesh;

/// How far under its own skin the body is drawn, in metres.
///
/// The cloth settles where the collision field reads zero, which is the skin
/// itself, so a body drawn at full size would fight the garment resting on it
/// for every pixel of the depth buffer. Two millimetres is under the 5 mm the
/// field is sampled at, so nothing the solver did is contradicted by it.
const SKIN: f32 = 0.002;

/// The body the cloth falls on, as the renderer draws it: interleaved
/// position, normal and colour, and the triangles over them.
///
/// Built by the interface rather than inside the renderer, because the two
/// bodies it can be — a generated person and the demo's ball — come from
/// different places, and neither belongs inside a buffer allocator.
pub struct Avatar {
    /// Nine floats per vertex: position, normal, colour.
    pub verts: Vec<f32>,
    /// Triangles indexing those vertices.
    pub idx: Vec<u32>,
}

impl Avatar {
    /// No body at all: what the viewport draws before one has been solved.
    pub fn none() -> Avatar {
        Avatar {
            verts: Vec::new(),
            idx: Vec::new(),
        }
    }

    /// A generated body, drawn just inside its own skin.
    pub fn body(mesh: &BodyMesh, color: [f32; 3]) -> Avatar {
        let n = mesh.vertex_count();
        let mut verts = Vec::with_capacity(n * 9);
        for i in 0..n {
            let (p, normal) = (&mesh.positions[i * 3..], &mesh.normals[i * 3..]);
            for c in 0..3 {
                verts.push(p[c] - normal[c] * SKIN);
            }
            verts.extend_from_slice(&normal[..3]);
            verts.extend_from_slice(&color);
        }
        Avatar {
            verts,
            idx: mesh.indices.clone(),
        }
    }

    /// How many vertices it carries.
    pub fn len(&self) -> usize {
        self.verts.len() / 9
    }
}
