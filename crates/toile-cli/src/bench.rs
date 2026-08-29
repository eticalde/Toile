//! `toile bench` — Spike 1 (issue #33): benchmark del kernel XPBD con el
//! patrón de acceso de una malla real.
//!
//! La grilla se construye ordenada y después se baraja la numeración de
//! vértices y el orden de las constraints con un LCG de semilla fija: mismo
//! gather/scatter que deja una CDT, y 100% reproducible.

use std::time::Instant;
use toile_sim::xpbd::{self, DistanceConstraints, SdfGrid, State};

/// PRNG determinista minúsculo (Knuth MMIX) — sin dependencia externa.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        ((self.next() >> 33) as usize) % n
    }
}

fn shuffle<T>(v: &mut [T], rng: &mut Lcg) {
    for i in (1..v.len()).rev() {
        v.swap(i, rng.below(i + 1));
    }
}

struct Scene {
    state: State,
    cons: DistanceConstraints,
    tris: Vec<u32>,
    sdf: SdfGrid,
}

/// Paño horizontal de ~`target` vértices a 5 mm de particle distance,
/// cayendo sobre una esfera SDF de 15 cm.
fn build(target: usize) -> Scene {
    const SPACING: f32 = 0.005;
    let w = (target as f64).sqrt() as usize;
    let h = target / w;
    let n = w * h;
    let mut rng = Lcg(0x0005_EED7_011E);

    // Permutación de vértices: perm[orden_grilla] = índice barajado.
    let mut perm: Vec<u32> = (0..n as u32).collect();
    shuffle(&mut perm, &mut rng);

    let mut state = State::new(n);
    let (ox, oz) = (w as f32 * SPACING * 0.5, h as f32 * SPACING * 0.5);
    for j in 0..h {
        for i in 0..w {
            let v = perm[j * w + i] as usize;
            state.px[v] = i as f32 * SPACING - ox;
            state.py[v] = 0.3;
            state.pz[v] = j as f32 * SPACING - oz;
        }
    }

    // Estructurales, shear y bending (compliance XPBD por tipo de arista —
    // el slot donde después vive la anisotropía urdimbre/trama).
    const C_STRUCT: f32 = 1.0e-8;
    const C_SHEAR: f32 = 5.0e-7;
    const C_BEND: f32 = 1.0e-5;
    let mut edges: Vec<(u32, u32, f32)> = Vec::new();
    let link =
        |edges: &mut Vec<(u32, u32, f32)>, i2: usize, j2: usize, i: usize, j: usize, c: f32| {
            edges.push((perm[j * w + i], perm[j2 * w + i2], c));
        };
    for j in 0..h {
        for i in 0..w {
            if i + 1 < w {
                link(&mut edges, i + 1, j, i, j, C_STRUCT);
            }
            if j + 1 < h {
                link(&mut edges, i, j + 1, i, j, C_STRUCT);
            }
            if i + 1 < w && j + 1 < h {
                link(&mut edges, i + 1, j + 1, i, j, C_SHEAR);
                link(&mut edges, i, j + 1, i + 1, j, C_SHEAR);
            }
            if i + 2 < w {
                link(&mut edges, i + 2, j, i, j, C_BEND);
            }
            if j + 2 < h {
                link(&mut edges, i, j + 2, i, j, C_BEND);
            }
        }
    }
    shuffle(&mut edges, &mut rng);

    let mut cons = DistanceConstraints {
        a: Vec::with_capacity(edges.len()),
        b: Vec::with_capacity(edges.len()),
        rest: Vec::with_capacity(edges.len()),
        compliance: Vec::with_capacity(edges.len()),
    };
    for (a, b, c) in &edges {
        let (ia, ib) = (*a as usize, *b as usize);
        let dx = state.px[ib] - state.px[ia];
        let dy = state.py[ib] - state.py[ia];
        let dz = state.pz[ib] - state.pz[ia];
        cons.a.push(*a);
        cons.b.push(*b);
        cons.rest.push((dx * dx + dy * dy + dz * dz).sqrt());
        cons.compliance.push(*c);
    }

    let mut tris: Vec<u32> = Vec::with_capacity((w - 1) * (h - 1) * 6);
    for j in 0..h - 1 {
        for i in 0..w - 1 {
            let (v00, v10) = (perm[j * w + i], perm[j * w + i + 1]);
            let (v01, v11) = (perm[(j + 1) * w + i], perm[(j + 1) * w + i + 1]);
            tris.extend_from_slice(&[v00, v10, v01, v10, v11, v01]);
        }
    }

    let sdf = SdfGrid::sphere(128, 1.4 / 127.0, [-0.7, -0.7, -0.7], [0.0, 0.0, 0.0], 0.15);

    Scene {
        state,
        cons,
        tris,
        sdf,
    }
}

const DT: f32 = 1.0 / 600.0; // 60 Hz visual · 10 substeps nominales
const WARMUP: usize = 30;
const TIMED: usize = 240;

/// Corre WARMUP+TIMED substeps sobre una escena fresca y devuelve
/// (ms por substep cronometrado, hash final de posiciones).
fn measure(target: usize, mut step: impl FnMut(&mut Scene)) -> (f64, u64) {
    let mut scene = build(target);
    for _ in 0..WARMUP {
        step(&mut scene);
    }
    let t = Instant::now();
    for _ in 0..TIMED {
        step(&mut scene);
    }
    let ms = t.elapsed().as_secs_f64() * 1000.0 / TIMED as f64;
    (ms, xpbd::position_hash(&scene.state))
}

fn run_size(target: usize) {
    let probe = build(target);
    let n = probe.state.len();
    let colored = xpbd::color_constraints(&probe.cons, n);
    let n_colors = colored.ranges.len();
    println!(
        "\n── {} vértices · {} constraints · {} triángulos · {} colores ──",
        n,
        probe.cons.len(),
        probe.tris.len() / 3,
        n_colors
    );

    // Baseline secuencial (orden original barajado).
    let (ms_mono, hash_mono) = measure(target, |s| {
        xpbd::substep(&mut s.state, &s.cons, &s.sdf, DT);
    });
    let (_, hash_mono2) = measure(target, |s| {
        xpbd::substep(&mut s.state, &s.cons, &s.sdf, DT);
    });

    // Coloring: barrido de hilos — mismo orden de colores en todos,
    // por lo tanto los bits deben ser idénticos entre sí (ADR §2.4).
    let all = std::thread::available_parallelism()
        .map(|v| v.get())
        .unwrap_or(8);
    let mut colored_runs = Vec::new();
    for t in [1usize, 4, 8, all] {
        if colored_runs.iter().any(|&(tt, _, _)| tt == t) {
            continue;
        }
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(t)
            .build()
            .unwrap();
        let (ms, hash) = measure(target, |s| {
            pool.install(|| xpbd::substep_colored(&mut s.state, &colored, &s.sdf, DT));
        });
        colored_runs.push((t, ms, hash));
    }

    let mut normals = vec![0.0f32; n * 3];
    let mut scene = build(target);
    for _ in 0..WARMUP {
        xpbd::substep(&mut scene.state, &scene.cons, &scene.sdf, DT);
    }
    let t = Instant::now();
    for _ in 0..60 {
        xpbd::vertex_normals(&scene.state, &scene.tris, &mut normals);
    }
    let ms_normals = t.elapsed().as_secs_f64() * 1000.0 / 60.0;

    let frame = |ms: f64| (16.6 / ms).floor();
    println!(
        "mono-hilo        {ms_mono:7.3} ms/substep  → {:.0} substeps/frame",
        frame(ms_mono)
    );
    for &(t, ms, _) in &colored_runs {
        println!(
            "coloring ×{t:<2}     {ms:7.3} ms/substep  → {:.0} substeps/frame · speedup {:.2}× vs mono",
            frame(ms),
            ms_mono / ms
        );
    }
    println!("normales         {ms_normals:7.3} ms (cadencia visual)");
    let par_ok = colored_runs.windows(2).all(|w| w[0].2 == w[1].2);
    println!(
        "determinismo     secuencial: {} · paralelo (todos los conteos de hilos): {}",
        if hash_mono == hash_mono2 {
            "OK"
        } else {
            "FALLÓ"
        },
        if par_ok {
            "OK (bit-idéntico)"
        } else {
            "FALLÓ"
        }
    );
}

pub fn run(args: &[String]) {
    let sizes: Vec<usize> = if let Some(i) = args.iter().position(|a| a == "--verts") {
        vec![args[i + 1].parse().expect("--verts N")]
    } else {
        vec![20_000, 50_000]
    };
    println!("toile bench — kernel XPBD, acceso barajado (Spike 1)");
    for s in sizes {
        run_size(s);
    }
}
