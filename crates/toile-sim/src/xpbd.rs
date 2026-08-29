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

/// Constraints reordenadas por color: dentro de un color, ningún par de
/// constraints comparte vértice — escrituras disjuntas, paralelizables sin
/// atomics y bit-idénticas con 1 u 8 hilos (ADR §2.4).
pub struct ColoredConstraints {
    pub cons: DistanceConstraints,
    pub ranges: Vec<std::ops::Range<usize>>,
}

/// Coloreo greedy determinista en el orden canónico de entrada: primer
/// color libre en ambos vértices (bitmask u64 → máximo 64 colores; una
/// malla de tela ronda los ~15).
pub fn color_constraints(cons: &DistanceConstraints, n_verts: usize) -> ColoredConstraints {
    let m = cons.len();
    let mut used: Vec<u64> = vec![0; n_verts];
    let mut color_of = vec![0u32; m];
    let mut n_colors = 0usize;
    for ((&a, &b), color) in cons.a.iter().zip(&cons.b).zip(color_of.iter_mut()) {
        let (a, b) = (a as usize, b as usize);
        let col = (!(used[a] | used[b])).trailing_zeros() as usize;
        assert!(col < 64, "grafo de constraints con más de 64 colores");
        used[a] |= 1 << col;
        used[b] |= 1 << col;
        *color = col as u32;
        n_colors = n_colors.max(col + 1);
    }

    // Grupos por color, y DENTRO de cada color orden por vértice menor:
    // recupera localidad de caché que el coloreo destruye. Determinista
    // (clave de orden total: vértice menor, luego índice original).
    let mut groups: Vec<Vec<usize>> = vec![Vec::new(); n_colors];
    for (i, &c) in color_of.iter().enumerate() {
        groups[c as usize].push(i);
    }
    for g in &mut groups {
        g.sort_by_key(|&i| (cons.a[i].min(cons.b[i]), i));
    }
    let mut cc = DistanceConstraints {
        a: Vec::with_capacity(m),
        b: Vec::with_capacity(m),
        rest: Vec::with_capacity(m),
        compliance: Vec::with_capacity(m),
    };
    let mut ranges = Vec::with_capacity(n_colors);
    for g in &groups {
        let start = cc.a.len();
        for &i in g {
            cc.a.push(cons.a[i]);
            cc.b.push(cons.b[i]);
            cc.rest.push(cons.rest[i]);
            cc.compliance.push(cons.compliance[i]);
        }
        ranges.push(start..cc.a.len());
    }
    ColoredConstraints { cons: cc, ranges }
}

/// Puntero crudo compartible entre hilos. La seguridad no la da el tipo:
/// la da el invariante de quien lo usa (índices disjuntos por hilo).
#[derive(Clone, Copy)]
struct Ptr(*mut f32);
unsafe impl Send for Ptr {}
unsafe impl Sync for Ptr {}

impl Ptr {
    /// El acceso pasa por un método sobre `self` para que las closures
    /// capturen el struct completo (Sync) y no el campo `*mut f32` suelto
    /// (captura disjunta de edition 2021+, que rompería el `Sync`).
    ///
    /// # Safety
    /// El llamador garantiza que `i` está en rango y que ningún otro hilo
    /// accede al mismo índice durante la fase.
    #[inline(always)]
    unsafe fn at(self, i: usize) -> *mut f32 {
        unsafe { self.0.add(i) }
    }
}

/// Substep XPBD paralelo: mismas fases que [`substep`], con las constraints
/// coloreadas. Corre en el pool de rayon activo — con 1 hilo o N produce
/// bits idénticos porque dentro de cada color las escrituras son disjuntas
/// y no hay reducciones.
pub fn substep_colored(state: &mut State, cc: &ColoredConstraints, sdf: &SdfGrid, dt: f32) {
    use rayon::prelude::*;
    const GRAVITY: f32 = -9.81;
    const MIN_CHUNK: usize = 4096;
    let n = state.len();

    let px = Ptr(state.px.as_mut_ptr());
    let py = Ptr(state.py.as_mut_ptr());
    let pz = Ptr(state.pz.as_mut_ptr());
    let vx = Ptr(state.vx.as_mut_ptr());
    let vy = Ptr(state.vy.as_mut_ptr());
    let vz = Ptr(state.vz.as_mut_ptr());
    let qx = Ptr(state.qx.as_mut_ptr());
    let qy = Ptr(state.qy.as_mut_ptr());
    let qz = Ptr(state.qz.as_mut_ptr());
    let inv_mass = &state.inv_mass;

    // Integración: cada índice lo escribe exactamente una iteración.
    (0..n)
        .into_par_iter()
        .with_min_len(MIN_CHUNK)
        .for_each(|i| unsafe {
            *qx.at(i) = *px.at(i);
            *qy.at(i) = *py.at(i);
            *qz.at(i) = *pz.at(i);
            if inv_mass[i] > 0.0 {
                *vy.at(i) += GRAVITY * dt;
                *px.at(i) += *vx.at(i) * dt;
                *py.at(i) += *vy.at(i) * dt;
                *pz.at(i) += *vz.at(i) * dt;
            }
        });

    // Constraints por color. SAFETY: dentro de un color ningún par de
    // constraints comparte vértice → lecturas y escrituras a px/py/pz de
    // esta fase son disjuntas entre iteraciones.
    let inv_dt2 = 1.0 / (dt * dt);
    let cons = &cc.cons;
    for r in &cc.ranges {
        (r.start..r.end)
            .into_par_iter()
            .with_min_len(MIN_CHUNK)
            .for_each(|c| unsafe {
                let (ia, ib) = (cons.a[c] as usize, cons.b[c] as usize);
                let (wa, wb) = (inv_mass[ia], inv_mass[ib]);
                let w = wa + wb;
                if w == 0.0 {
                    return;
                }
                let dx = *px.at(ib) - *px.at(ia);
                let dy = *py.at(ib) - *py.at(ia);
                let dz = *pz.at(ib) - *pz.at(ia);
                let len = (dx * dx + dy * dy + dz * dz).sqrt();
                if len <= 1.0e-9 {
                    return;
                }
                let alpha = cons.compliance[c] * inv_dt2;
                let corr = (len - cons.rest[c]) / ((w + alpha) * len);
                let (sx, sy, sz) = (corr * dx, corr * dy, corr * dz);
                *px.at(ia) += wa * sx;
                *py.at(ia) += wa * sy;
                *pz.at(ia) += wa * sz;
                *px.at(ib) -= wb * sx;
                *py.at(ib) -= wb * sy;
                *pz.at(ib) -= wb * sz;
            });
    }

    // Colisión SDF y velocidades: por índice, disjuntas.
    let eps = sdf.cell * 0.5;
    (0..n)
        .into_par_iter()
        .with_min_len(MIN_CHUNK)
        .for_each(|i| unsafe {
            let (x, y, z) = (*px.at(i), *py.at(i), *pz.at(i));
            let d = sdf.sample(x, y, z);
            if d < 0.0 {
                let gx = sdf.sample(x + eps, y, z) - d;
                let gy = sdf.sample(x, y + eps, z) - d;
                let gz = sdf.sample(x, y, z + eps) - d;
                let glen = (gx * gx + gy * gy + gz * gz).sqrt().max(1.0e-9);
                let push = -d / glen;
                *px.at(i) += gx * push;
                *py.at(i) += gy * push;
                *pz.at(i) += gz * push;
            }
        });

    let inv_dt = 1.0 / dt;
    const DAMPING: f32 = 0.999;
    (0..n)
        .into_par_iter()
        .with_min_len(MIN_CHUNK)
        .for_each(|i| unsafe {
            *vx.at(i) = (*px.at(i) - *qx.at(i)) * inv_dt * DAMPING;
            *vy.at(i) = (*py.at(i) - *qy.at(i)) * inv_dt * DAMPING;
            *vz.at(i) = (*pz.at(i) - *qz.at(i)) * inv_dt * DAMPING;
        });
}

/// Variante SIMD de [`substep_colored`]: misma estructura, con la
/// aritmética de constraints en lotes de 8 (`wide::f32x8`) — gathers y
/// scatters escalares (NEON/AVX2 sin gather por hardware), matemática
/// vectorizada. Cada lane ejecuta las mismas operaciones IEEE que el
/// camino escalar, así que el resultado debe ser bit-idéntico a él.
pub fn substep_colored_simd(state: &mut State, cc: &ColoredConstraints, sdf: &SdfGrid, dt: f32) {
    use rayon::prelude::*;
    use wide::{CmpGt, f32x8};
    const GRAVITY: f32 = -9.81;
    const MIN_CHUNK: usize = 4096;
    let n = state.len();

    let px = Ptr(state.px.as_mut_ptr());
    let py = Ptr(state.py.as_mut_ptr());
    let pz = Ptr(state.pz.as_mut_ptr());
    let vx = Ptr(state.vx.as_mut_ptr());
    let vy = Ptr(state.vy.as_mut_ptr());
    let vz = Ptr(state.vz.as_mut_ptr());
    let qx = Ptr(state.qx.as_mut_ptr());
    let qy = Ptr(state.qy.as_mut_ptr());
    let qz = Ptr(state.qz.as_mut_ptr());
    let inv_mass = &state.inv_mass;

    // Integración: contigua → SIMD directo por bloques de 8. En la escena
    // todos los inv_mass son > 0; el lane inactivo se maneja escalar en la
    // cola. SAFETY: bloques disjuntos por iteración.
    let blocks = n / 8;
    (0..blocks)
        .into_par_iter()
        .with_min_len(MIN_CHUNK / 8)
        .for_each(|blk| unsafe {
            let i = blk * 8;
            let load = |p: Ptr| f32x8::from(std::array::from_fn(|l| *p.at(i + l)));
            let store = |p: Ptr, v: f32x8| {
                let a = v.to_array();
                for (l, x) in a.iter().enumerate() {
                    *p.at(i + l) = *x;
                }
            };
            let (pxv, pyv, pzv) = (load(px), load(py), load(pz));
            store(qx, pxv);
            store(qy, pyv);
            store(qz, pzv);
            let w = f32x8::from(std::array::from_fn(|l| inv_mass[i + l]));
            let active = w.cmp_gt(f32x8::splat(0.0));
            let vyv = load(vy) + active.blend(f32x8::splat(GRAVITY * dt), f32x8::splat(0.0));
            store(vy, vyv);
            let dtv = f32x8::splat(dt);
            store(px, pxv + active.blend(load(vx) * dtv, f32x8::splat(0.0)));
            store(py, pyv + active.blend(vyv * dtv, f32x8::splat(0.0)));
            store(pz, pzv + active.blend(load(vz) * dtv, f32x8::splat(0.0)));
        });
    for (i, &im) in inv_mass.iter().enumerate().skip(blocks * 8) {
        unsafe {
            *qx.at(i) = *px.at(i);
            *qy.at(i) = *py.at(i);
            *qz.at(i) = *pz.at(i);
            if im > 0.0 {
                *vy.at(i) += GRAVITY * dt;
                *px.at(i) += *vx.at(i) * dt;
                *py.at(i) += *vy.at(i) * dt;
                *pz.at(i) += *vz.at(i) * dt;
            }
        }
    }

    // Constraints por color, en lotes de 8. SAFETY: dentro de un color las
    // constraints no comparten vértices → los 8 gathers de un lote leen
    // posiciones que ningún otro lote del color escribe, y los scatters
    // son disjuntos.
    let inv_dt2 = f32x8::splat(1.0 / (dt * dt));
    let cons = &cc.cons;
    for r in &cc.ranges {
        let m = r.end - r.start;
        let batches = m / 8;
        (0..batches)
            .into_par_iter()
            .with_min_len(MIN_CHUNK / 8)
            .for_each(|bt| unsafe {
                let c0 = r.start + bt * 8;
                let ia: [usize; 8] = std::array::from_fn(|l| cons.a[c0 + l] as usize);
                let ib: [usize; 8] = std::array::from_fn(|l| cons.b[c0 + l] as usize);
                let gx =
                    |p: Ptr, idx: &[usize; 8]| f32x8::from(std::array::from_fn(|l| *p.at(idx[l])));
                let wa = f32x8::from(std::array::from_fn(|l| inv_mass[ia[l]]));
                let wb = f32x8::from(std::array::from_fn(|l| inv_mass[ib[l]]));
                let w = wa + wb;
                let dx = gx(px, &ib) - gx(px, &ia);
                let dy = gx(py, &ib) - gx(py, &ia);
                let dz = gx(pz, &ib) - gx(pz, &ia);
                let len = (dx * dx + dy * dy + dz * dz).sqrt();
                let rest = f32x8::from(std::array::from_fn(|l| cons.rest[c0 + l]));
                let compliance = f32x8::from(std::array::from_fn(|l| cons.compliance[c0 + l]));
                let alpha = compliance * inv_dt2;
                let valid = w.cmp_gt(f32x8::splat(0.0)) & len.cmp_gt(f32x8::splat(1.0e-9));
                let corr = valid.blend((len - rest) / ((w + alpha) * len), f32x8::splat(0.0));
                let sx = (corr * dx).to_array();
                let sy = (corr * dy).to_array();
                let sz = (corr * dz).to_array();
                let waa = wa.to_array();
                let wba = wb.to_array();
                for l in 0..8 {
                    *px.at(ia[l]) += waa[l] * sx[l];
                    *py.at(ia[l]) += waa[l] * sy[l];
                    *pz.at(ia[l]) += waa[l] * sz[l];
                    *px.at(ib[l]) -= wba[l] * sx[l];
                    *py.at(ib[l]) -= wba[l] * sy[l];
                    *pz.at(ib[l]) -= wba[l] * sz[l];
                }
            });
        // Cola escalar del color, misma fórmula que substep_colored.
        for c in r.start + batches * 8..r.end {
            unsafe {
                let (ia, ib) = (cons.a[c] as usize, cons.b[c] as usize);
                let (wa, wb) = (inv_mass[ia], inv_mass[ib]);
                let w = wa + wb;
                if w == 0.0 {
                    continue;
                }
                let dx = *px.at(ib) - *px.at(ia);
                let dy = *py.at(ib) - *py.at(ia);
                let dz = *pz.at(ib) - *pz.at(ia);
                let len = (dx * dx + dy * dy + dz * dz).sqrt();
                if len <= 1.0e-9 {
                    continue;
                }
                let alpha = cons.compliance[c] * (1.0 / (dt * dt));
                let corr = (len - cons.rest[c]) / ((w + alpha) * len);
                let (sx, sy, sz) = (corr * dx, corr * dy, corr * dz);
                *px.at(ia) += wa * sx;
                *py.at(ia) += wa * sy;
                *pz.at(ia) += wa * sz;
                *px.at(ib) -= wb * sx;
                *py.at(ib) -= wb * sy;
                *pz.at(ib) -= wb * sz;
            }
        }
    }

    // Colisión SDF (escalar: trilineal = gathers dispersos) y velocidades.
    let eps = sdf.cell * 0.5;
    (0..n)
        .into_par_iter()
        .with_min_len(MIN_CHUNK)
        .for_each(|i| unsafe {
            let (x, y, z) = (*px.at(i), *py.at(i), *pz.at(i));
            let d = sdf.sample(x, y, z);
            if d < 0.0 {
                let gx = sdf.sample(x + eps, y, z) - d;
                let gy = sdf.sample(x, y + eps, z) - d;
                let gz = sdf.sample(x, y, z + eps) - d;
                let glen = (gx * gx + gy * gy + gz * gz).sqrt().max(1.0e-9);
                let push = -d / glen;
                *px.at(i) += gx * push;
                *py.at(i) += gy * push;
                *pz.at(i) += gz * push;
            }
        });

    let inv_dt = 1.0 / dt;
    const DAMPING: f32 = 0.999;
    (0..n)
        .into_par_iter()
        .with_min_len(MIN_CHUNK)
        .for_each(|i| unsafe {
            *vx.at(i) = (*px.at(i) - *qx.at(i)) * inv_dt * DAMPING;
            *vy.at(i) = (*py.at(i) - *qy.at(i)) * inv_dt * DAMPING;
            *vz.at(i) = (*pz.at(i) - *qz.at(i)) * inv_dt * DAMPING;
        });
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

/// Energía cinética total (masa unitaria) — el sensor del kinetic damping.
pub fn kinetic_energy(state: &State) -> f32 {
    let mut e = 0.0f32;
    for i in 0..state.len() {
        e += state.vx[i] * state.vx[i] + state.vy[i] * state.vy[i] + state.vz[i] * state.vz[i];
    }
    e * 0.5
}

/// Kinetic damping (Provot): al detectar un pico de energía cinética se
/// anulan todas las velocidades — la tela se asienta en el equilibrio
/// estático en vez de columpiarse. Mecanismo explícito de convergencia
/// del ADR §2.3. Determinista.
pub fn zero_velocities(state: &mut State) {
    state.vx.fill(0.0);
    state.vy.fill(0.0);
    state.vz.fill(0.0);
}
