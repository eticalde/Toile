/// A UV sphere as interleaved position, normal and colour.
pub fn uv_sphere(r: f32, seg: u32, rings: u32) -> (Vec<f32>, Vec<u32>) {
    const COLOR: [f32; 3] = [0.30, 0.33, 0.32];
    let mut v = Vec::new();
    for j in 0..=rings {
        let phi = std::f32::consts::PI * j as f32 / rings as f32;
        let (sp, cp) = phi.sin_cos();
        for i in 0..=seg {
            let theta = std::f32::consts::TAU * i as f32 / seg as f32;
            let (st, ct) = theta.sin_cos();
            let n = [sp * ct, cp, sp * st];
            v.extend_from_slice(&[r * n[0], r * n[1], r * n[2], n[0], n[1], n[2]]);
            v.extend_from_slice(&COLOR);
        }
    }
    let mut idx = Vec::new();
    let stride = seg + 1;
    for j in 0..rings {
        for i in 0..seg {
            let a = j * stride + i;
            let b = a + stride;
            idx.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    (v, idx)
}
