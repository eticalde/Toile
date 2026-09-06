/// Position, normal and colour, interleaved.
pub const VERTEX_STRIDE: u64 = 9 * 4;

/// Byte sizes and the assembled index list for one topology of the scene.
///
/// A mesh swap changes the cloth's vertex and triangle counts at once, and
/// everything the GPU side has to redo follows from these numbers. They are
/// plain arithmetic, kept apart from the device so a swap can be exercised
/// without a window.
pub struct BufferPlan {
    /// Bytes the vertex buffer needs for the cloth plus the avatar.
    pub vbuf_bytes: u64,
    /// Where the avatar's vertices start: right after the cloth's.
    pub sphere_offset: u64,
    /// The cloth's triangles, then the avatar's rebased past the cloth.
    pub indices: Vec<u32>,
}

/// Lays out both mesh buffers for a cloth of `n_cloth_verts` vertices.
pub fn plan(
    n_cloth_verts: usize,
    cloth_tris: &[u32],
    sphere_verts: &[f32],
    sphere_idx: &[u32],
) -> BufferPlan {
    let n_sphere_verts = sphere_verts.len() / 9;
    let mut indices = cloth_tris.to_vec();
    indices.extend(sphere_idx.iter().map(|&i| i + n_cloth_verts as u32));
    BufferPlan {
        vbuf_bytes: (n_cloth_verts + n_sphere_verts) as u64 * VERTEX_STRIDE,
        sphere_offset: n_cloth_verts as u64 * VERTEX_STRIDE,
        indices,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A topology swap moves both counts, so it has to resize both buffers
    /// together and rebase the avatar's indices past the new cloth, not the
    /// old one.
    #[test]
    fn resize_reallocates_both_buffers() {
        let sphere_verts = vec![0.0; 4 * 9];
        let sphere_idx = [0, 1, 2, 0, 2, 3];
        let before = plan(100, &[0, 1, 2], &sphere_verts, &sphere_idx);
        let after = plan(250, &[0, 1, 2, 3, 4, 5], &sphere_verts, &sphere_idx);
        assert_ne!(after.vbuf_bytes, before.vbuf_bytes);
        assert_ne!(after.indices.len(), before.indices.len());
        assert_eq!(after.vbuf_bytes, 254 * VERTEX_STRIDE);
        assert_eq!(after.sphere_offset, 250 * VERTEX_STRIDE);
        assert_eq!(after.indices[..6], [0, 1, 2, 3, 4, 5]);
        assert_eq!(after.indices[6..], [250, 251, 252, 250, 252, 253]);
    }
}
