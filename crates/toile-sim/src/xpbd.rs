//! Kernel XPBD small-steps — Spike 1 (issue #33).
//!
//! Baseline mono-hilo con el layout definitivo: SoA de `Vec<f32>` planos.
//! Los índices de vértices y el orden de las constraints llegan barajados
//! desde el harness para reproducir el patrón gather/scatter de una malla
//! CDT real (un microbench con acceso denso mentiría — ADR §5.1).

/// Estado de las partículas en Structure-of-Arrays.
///
/// `qx/qy/qz` guardan la posición previa al substep: la velocidad se deriva
/// de ahí (PBD clásico), no se integra aparte.
pub struct State {
    pub px: Vec<f32>,
    pub py: Vec<f32>,
    pub pz: Vec<f32>,
    pub vx: Vec<f32>,
    pub vy: Vec<f32>,
    pub vz: Vec<f32>,
    pub qx: Vec<f32>,
    pub qy: Vec<f32>,
    pub qz: Vec<f32>,
    pub inv_mass: Vec<f32>,
}

impl State {
    pub fn new(n: usize) -> Self {
        Self {
            px: vec![0.0; n],
            py: vec![0.0; n],
            pz: vec![0.0; n],
            vx: vec![0.0; n],
            vy: vec![0.0; n],
            vz: vec![0.0; n],
            qx: vec![0.0; n],
            qy: vec![0.0; n],
            qz: vec![0.0; n],
            inv_mass: vec![1.0; n],
        }
    }

    pub fn len(&self) -> usize {
        self.px.len()
    }

    pub fn is_empty(&self) -> bool {
        self.px.is_empty()
    }
}

/// Constraints de distancia en SoA: la arista `i` une `a[i]`–`b[i]`.
pub struct DistanceConstraints {
    pub a: Vec<u32>,
    pub b: Vec<u32>,
    pub rest: Vec<f32>,
    /// Compliance XPBD por arista (la anisotropía urdimbre/trama vive aquí).
    pub compliance: Vec<f32>,
}

impl DistanceConstraints {
    pub fn len(&self) -> usize {
        self.a.len()
    }

    pub fn is_empty(&self) -> bool {
        self.a.is_empty()
    }
}

/// SDF en grilla regular con sample trilineal — colisión contra el avatar.
pub struct SdfGrid {
    pub dim: usize,
    pub cell: f32,
    pub origin: [f32; 3],
    pub data: Vec<f32>,
}

impl SdfGrid {
    /// SDF sintético de esfera: suficiente para medir el costo real del
    /// acceso a la grilla (8 MB para 128³, no cabe en L2).
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

/// Un substep XPBD completo: integración, 1 iteración de constraints
/// (small steps: N substeps de 1 iteración), colisión SDF y velocidades.
pub fn substep(state: &mut State, cons: &DistanceConstraints, sdf: &SdfGrid, dt: f32) {
    const GRAVITY: f32 = -9.81;
    let n = state.len();

    // Integración semi-implícita + guardar posición previa.
    for i in 0..n {
        state.qx[i] = state.px[i];
        state.qy[i] = state.py[i];
        state.qz[i] = state.pz[i];
        if state.inv_mass[i] > 0.0 {
            state.vy[i] += GRAVITY * dt;
            state.px[i] += state.vx[i] * dt;
            state.py[i] += state.vy[i] * dt;
            state.pz[i] += state.vz[i] * dt;
        }
    }

    // Gauss-Seidel secuencial sobre las constraints (orden barajado =
    // gather/scatter real). Con 1 iteración por substep, lambda parte de 0
    // en cada substep y no necesita acumularse.
    let inv_dt2 = 1.0 / (dt * dt);
    for c in 0..cons.len() {
        let (ia, ib) = (cons.a[c] as usize, cons.b[c] as usize);
        let (wa, wb) = (state.inv_mass[ia], state.inv_mass[ib]);
        let w = wa + wb;
        if w == 0.0 {
            continue;
        }
        let dx = state.px[ib] - state.px[ia];
        let dy = state.py[ib] - state.py[ia];
        let dz = state.pz[ib] - state.pz[ia];
        let len = (dx * dx + dy * dy + dz * dz).sqrt();
        if len <= 1.0e-9 {
            continue;
        }
        let alpha = cons.compliance[c] * inv_dt2;
        let corr = (len - cons.rest[c]) / ((w + alpha) * len);
        let (sx, sy, sz) = (corr * dx, corr * dy, corr * dz);
        state.px[ia] += wa * sx;
        state.py[ia] += wa * sy;
        state.pz[ia] += wa * sz;
        state.px[ib] -= wb * sx;
        state.py[ib] -= wb * sy;
        state.pz[ib] -= wb * sz;
    }

    // Colisión contra el SDF: proyección fuera + gradiente por diferencias
    // finitas (4 samples por vértice en contacto, 1 si está libre).
    let eps = sdf.cell * 0.5;
    for i in 0..n {
        let (x, y, z) = (state.px[i], state.py[i], state.pz[i]);
        let d = sdf.sample(x, y, z);
        if d < 0.0 {
            let gx = sdf.sample(x + eps, y, z) - d;
            let gy = sdf.sample(x, y + eps, z) - d;
            let gz = sdf.sample(x, y, z + eps) - d;
            let glen = (gx * gx + gy * gy + gz * gz).sqrt().max(1.0e-9);
            let push = -d / glen;
            state.px[i] += gx * push;
            state.py[i] += gy * push;
            state.pz[i] += gz * push;
        }
    }

    // Velocidades desde el delta de posición, con damping suave.
    let inv_dt = 1.0 / dt;
    const DAMPING: f32 = 0.999;
    for i in 0..n {
        state.vx[i] = (state.px[i] - state.qx[i]) * inv_dt * DAMPING;
        state.vy[i] = (state.py[i] - state.qy[i]) * inv_dt * DAMPING;
        state.vz[i] = (state.pz[i] - state.qz[i]) * inv_dt * DAMPING;
    }
}

/// Normales por vértice: acumulación de cruces por triángulo (scatter) y
/// normalización. Corre a cadencia visual (60 Hz), no por substep.
pub fn vertex_normals(state: &State, tris: &[u32], out: &mut [f32]) {
    out.fill(0.0);
    for t in tris.as_chunks::<3>().0 {
        let (a, b, c) = (t[0] as usize, t[1] as usize, t[2] as usize);
        let (e1x, e1y, e1z) = (
            state.px[b] - state.px[a],
            state.py[b] - state.py[a],
            state.pz[b] - state.pz[a],
        );
        let (e2x, e2y, e2z) = (
            state.px[c] - state.px[a],
            state.py[c] - state.py[a],
            state.pz[c] - state.pz[a],
        );
        let nx = e1y * e2z - e1z * e2y;
        let ny = e1z * e2x - e1x * e2z;
        let nz = e1x * e2y - e1y * e2x;
        for &v in &[a, b, c] {
            out[v * 3] += nx;
            out[v * 3 + 1] += ny;
            out[v * 3 + 2] += nz;
        }
    }
    for n in out.as_chunks_mut::<3>().0 {
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1.0e-9);
        n[0] /= len;
        n[1] /= len;
        n[2] /= len;
    }
}

/// Hash FNV-1a de los bits de las posiciones — el germen de los goldens de
/// determinismo: misma escena, mismos substeps → mismo hash, siempre.
pub fn position_hash(state: &State) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    let mut eat = |v: &[f32]| {
        for x in v {
            h = (h ^ u64::from(x.to_bits())).wrapping_mul(0x100000001b3);
        }
    };
    eat(&state.px);
    eat(&state.py);
    eat(&state.pz);
    h
}
