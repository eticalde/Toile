use super::avatar::Avatar;

/// Position, normal and colour, interleaved.
pub const VERTEX_STRIDE: u64 = 9 * 4;

/// Byte sizes and the assembled index list for one topology of the scene.
///
/// A mesh swap changes the cloth's vertex and triangle counts at once, and a
/// body re-solved under the cloth changes the other half; everything the GPU
/// side has to redo follows from these numbers. They are plain arithmetic,
/// kept apart from the device so either change can be exercised without a
/// window.
pub struct BufferPlan {
    /// Bytes the vertex buffer needs for the cloth plus the body.
    pub vbuf_bytes: u64,
    /// Where the body's vertices start: right after the cloth's.
    pub avatar_offset: u64,
    /// The cloth's triangles, then the body's rebased past the cloth.
    pub indices: Vec<u32>,
}

/// Lays out both mesh buffers for a cloth of `n_cloth_verts` vertices.
pub fn plan(n_cloth_verts: usize, cloth_tris: &[u32], avatar: &Avatar) -> BufferPlan {
    let mut indices = cloth_tris.to_vec();
    indices.extend(avatar.idx.iter().map(|&i| i + n_cloth_verts as u32));
    BufferPlan {
        vbuf_bytes: (n_cloth_verts + avatar.len()) as u64 * VERTEX_STRIDE,
        avatar_offset: n_cloth_verts as u64 * VERTEX_STRIDE,
        indices,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A body of four vertices, standing in for whichever one is on the view.
    fn avatar() -> Avatar {
        Avatar {
            verts: vec![0.0; 4 * 9],
            idx: vec![0, 1, 2, 0, 2, 3],
        }
    }

    /// A topology swap moves both counts, so it has to resize both buffers
    /// together and rebase the body's indices past the new cloth, not the
    /// old one.
    #[test]
    fn resize_reallocates_both_buffers() {
        let before = plan(100, &[0, 1, 2], &avatar());
        let after = plan(250, &[0, 1, 2, 3, 4, 5], &avatar());
        assert_ne!(after.vbuf_bytes, before.vbuf_bytes);
        assert_ne!(after.indices.len(), before.indices.len());
        assert_eq!(after.vbuf_bytes, 254 * VERTEX_STRIDE);
        assert_eq!(after.avatar_offset, 250 * VERTEX_STRIDE);
        assert_eq!(after.indices[..6], [0, 1, 2, 3, 4, 5]);
        assert_eq!(after.indices[6..], [250, 251, 252, 250, 252, 253]);
    }

    /// A body swapped for one of another size rebases its indices the same
    /// way: the cloth's own triangles are untouched by it.
    #[test]
    fn another_body_rebases_past_the_same_cloth() {
        let bigger = Avatar {
            verts: vec![0.0; 6 * 9],
            idx: vec![0, 1, 2, 3, 4, 5],
        };
        let plan = plan(100, &[0, 1, 2], &bigger);
        assert_eq!(plan.vbuf_bytes, 106 * VERTEX_STRIDE);
        assert_eq!(plan.avatar_offset, 100 * VERTEX_STRIDE);
        assert_eq!(plan.indices[..3], [0, 1, 2]);
        assert_eq!(plan.indices[3..], [100, 101, 102, 103, 104, 105]);
    }

    /// Before a body is solved there is none, and the plan is the cloth's
    /// alone rather than a buffer nobody sized.
    #[test]
    fn with_no_body_the_plan_is_the_cloths_own() {
        let plan = plan(100, &[0, 1, 2], &Avatar::none());
        assert_eq!(plan.vbuf_bytes, 100 * VERTEX_STRIDE);
        assert_eq!(plan.indices, [0, 1, 2]);
    }
}
