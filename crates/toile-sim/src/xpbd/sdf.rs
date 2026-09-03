/// A signed distance field on a regular grid, sampled trilinearly.
#[allow(
    missing_docs,
    reason = "SoA buffers are named by their axis; a doc
    per field would only restate the name"
)]
#[derive(Debug, Clone)]
pub struct SdfGrid {
    pub dim: usize,
    pub cell: f32,
    pub origin: [f32; 3],
    pub data: Vec<f32>,
}

impl SdfGrid {
    /// A sphere, baked analytically.
    ///
    /// Stands in for the avatar until glTF baking exists, and is deliberately
    /// full resolution: at 128³ the grid is 8 MB and misses L2, which is the
    /// cost the real avatar will have too.
    pub fn sphere(dim: usize, cell: f32, origin: [f32; 3], center: [f32; 3], radius: f32) -> Self {
        let mut data = vec![0.0f32; dim * dim * dim];
        for k in 0..dim {
            for j in 0..dim {
                for i in 0..dim {
                    let x = origin[0] + i as f32 * cell - center[0];
                    let y = origin[1] + j as f32 * cell - center[1];
                    let z = origin[2] + k as f32 * cell - center[2];
                    data[(k * dim + j) * dim + i] = (x * x + y * y + z * z).sqrt() - radius;
                }
            }
        }
        Self {
            dim,
            cell,
            origin,
            data,
        }
    }

    /// Distance at a point; negative inside. Coordinates are clamped to the
    /// grid, so the field reads as extended rather than out of bounds.
    #[inline]
    pub fn sample(&self, x: f32, y: f32, z: f32) -> f32 {
        let d = self.dim;
        let fx = ((x - self.origin[0]) / self.cell).clamp(0.0, (d - 2) as f32);
        let fy = ((y - self.origin[1]) / self.cell).clamp(0.0, (d - 2) as f32);
        let fz = ((z - self.origin[2]) / self.cell).clamp(0.0, (d - 2) as f32);
        let (i, j, k) = (fx as usize, fy as usize, fz as usize);
        let (tx, ty, tz) = (fx - i as f32, fy - j as f32, fz - k as f32);
        let at = |i: usize, j: usize, k: usize| self.data[(k * d + j) * d + i];
        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let c00 = lerp(at(i, j, k), at(i + 1, j, k), tx);
        let c10 = lerp(at(i, j + 1, k), at(i + 1, j + 1, k), tx);
        let c01 = lerp(at(i, j, k + 1), at(i + 1, j, k + 1), tx);
        let c11 = lerp(at(i, j + 1, k + 1), at(i + 1, j + 1, k + 1), tx);
        lerp(lerp(c00, c10, ty), lerp(c01, c11, ty), tz)
    }
}
