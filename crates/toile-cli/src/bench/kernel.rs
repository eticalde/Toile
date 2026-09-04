use std::time::Instant;

use toile_sim::xpbd::{self, DistanceConstraints, SdfGrid, Seams, State};

use super::scene::{DT, Lcg, same_bits, shuffle};

const WARMUP: usize = 30;
const TIMED: usize = 240;

/// A flat sheet of roughly `target` vertices at 5 mm particle distance,
/// falling onto a 15 cm sphere.
pub struct Scene {
    pub state: State,
    pub cons: DistanceConstraints,
    pub tris: Vec<u32>,
    pub sdf: SdfGrid,
}

/// Builds the sheet with its vertex numbering and constraint order shuffled.
///
/// The shuffle is the point: a real CDT mesh leaves scattered gather/scatter,
/// and a benchmark over a densely ordered grid would report a speed the
/// product never sees.
pub fn build(target: usize) -> Scene {
    const SPACING: f32 = 0.005;
    const C_STRUCT: f32 = 1.0e-8;
    const C_SHEAR: f32 = 5.0e-7;
    const C_BEND: f32 = 1.0e-5;

    let w = (target as f64).sqrt() as usize;
    let h = target / w;
    let n = w * h;
    let mut rng = Lcg(0x0005_EED7_011E);

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

    let mut edges: Vec<(u32, u32, f32)> = Vec::new();
    let mut link = |i2: usize, j2: usize, i: usize, j: usize, c: f32| {
        edges.push((perm[j * w + i], perm[j2 * w + i2], c));
    };
    for j in 0..h {
        for i in 0..w {
            if i + 1 < w {
                link(i + 1, j, i, j, C_STRUCT);
            }
            if j + 1 < h {
                link(i, j + 1, i, j, C_STRUCT);
            }
            if i + 1 < w && j + 1 < h {
                link(i + 1, j + 1, i, j, C_SHEAR);
                link(i, j + 1, i + 1, j, C_SHEAR);
            }
            if i + 2 < w {
                link(i + 2, j, i, j, C_BEND);
            }
            if j + 2 < h {
                link(i, j + 2, i, j, C_BEND);
            }
        }
    }
    shuffle(&mut edges, &mut rng);

    let mut cons = DistanceConstraints {
        a: Vec::with_capacity(edges.len()),
        b: Vec::with_capacity(edges.len()),
        rest: Vec::with_capacity(edges.len()),
        compliance: Vec::with_capacity(edges.len()),
        strain_limit: 0.0,
        strain_sweeps: 0,
    };
    for (a, b, c) in edges {
        let (ia, ib) = (a as usize, b as usize);
        let (dx, dy, dz) = (
            state.px[ib] - state.px[ia],
            state.py[ib] - state.py[ia],
            state.pz[ib] - state.pz[ia],
        );
        cons.a.push(a);
        cons.b.push(b);
        cons.rest.push((dx * dx + dy * dy + dz * dz).sqrt());
        cons.compliance.push(c);
    }

    let mut tris = Vec::with_capacity((w - 1) * (h - 1) * 6);
    for j in 0..h - 1 {
        for i in 0..w - 1 {
            let (a, b, c, d) = (
                perm[j * w + i],
                perm[j * w + i + 1],
                perm[(j + 1) * w + i],
                perm[(j + 1) * w + i + 1],
            );
            tris.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }

    Scene {
        state,
        cons,
        tris,
        sdf: toile_engine::demo::avatar_sdf(),
    }
}

/// Milliseconds per timed substep, and the final position hash.
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

pub fn run(args: &[String]) {
    let sizes: Vec<usize> = if let Some(i) = args.iter().position(|a| a == "--verts") {
        vec![args[i + 1].parse().expect("--verts N")]
    } else {
        vec![20_000, 50_000]
    };
    println!("toile bench — kernel XPBD, acceso barajado");
    for s in sizes {
        run_size(s);
    }
    run_mesh();
}

/// One timing run per solver path at a given vertex count.
struct Timings {
    mono: f64,
    mono_hashes: (u64, u64),
    colored: Vec<(usize, f64, u64)>,
    simd: Vec<(usize, f64, u64)>,
    normals: f64,
}

fn time_paths(target: usize, colored: &xpbd::ColoredConstraints) -> Timings {
    let no_seams = Seams::default();
    let (mono, h1) = measure(target, |s| {
        xpbd::substep(&mut s.state, &s.cons, &no_seams, &s.sdf, DT);
    });
    let (_, h2) = measure(target, |s| {
        xpbd::substep(&mut s.state, &s.cons, &no_seams, &s.sdf, DT);
    });

    let all = std::thread::available_parallelism().map_or(8, std::num::NonZero::get);
    let mut runs = Vec::new();
    for t in [1usize, 4, 8, all] {
        if runs.iter().any(|&(tt, _, _)| tt == t) {
            continue;
        }
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(t)
            .build()
            .unwrap();
        let (ms, hash) = measure(target, |s| {
            pool.install(|| xpbd::substep_colored(&mut s.state, colored, &s.sdf, DT));
        });
        runs.push((t, ms, hash));
    }

    let mut simd = Vec::new();
    for t in [1usize, 4] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(t)
            .build()
            .unwrap();
        let (ms, hash) = measure(target, |s| {
            pool.install(|| xpbd::substep_colored_simd(&mut s.state, colored, &s.sdf, DT));
        });
        simd.push((t, ms, hash));
    }

    Timings {
        mono,
        mono_hashes: (h1, h2),
        colored: runs,
        simd,
        normals: time_normals(target),
    }
}

fn time_normals(target: usize) -> f64 {
    let no_seams = Seams::default();
    let mut scene = build(target);
    let mut normals = vec![0.0f32; scene.state.len() * 3];
    for _ in 0..WARMUP {
        xpbd::substep(&mut scene.state, &scene.cons, &no_seams, &scene.sdf, DT);
    }
    let t = Instant::now();
    for _ in 0..60 {
        xpbd::vertex_normals(&scene.state, &scene.tris, &mut normals);
    }
    t.elapsed().as_secs_f64() * 1000.0 / 60.0
}

fn run_size(target: usize) {
    let probe = build(target);
    let n = probe.state.len();
    let colored = xpbd::color_constraints(&probe.cons, n);
    println!(
        "\n── {} vértices · {} constraints · {} triángulos · {} colores ──",
        n,
        probe.cons.len(),
        probe.tris.len() / 3,
        colored.ranges.len()
    );

    let t = time_paths(target, &colored);
    let frame = |ms: f64| (16.6 / ms).floor();
    println!(
        "mono-hilo        {:7.3} ms/substep  → {:.0} substeps/frame",
        t.mono,
        frame(t.mono)
    );
    for &(threads, ms, _) in &t.colored {
        println!(
            "coloring ×{threads:<2}     {ms:7.3} ms/substep  → {:.0} substeps/frame · speedup {:.2}× vs mono",
            frame(ms),
            t.mono / ms
        );
    }
    for &(threads, ms, _) in &t.simd {
        println!(
            "simd f32x8 ×{threads:<2}   {ms:7.3} ms/substep  → {:.0} substeps/frame · speedup {:.2}× vs mono",
            frame(ms),
            t.mono / ms
        );
    }
    println!("normales         {:7.3} ms (cadencia visual)", t.normals);

    let par_ok = t.colored.windows(2).all(|w| w[0].2 == w[1].2);
    let simd_ok = t.simd.iter().all(|&(_, _, h)| h == t.colored[0].2);
    println!(
        "determinismo     secuencial: {} · paralelo: {} · simd vs escalar: {}",
        same_bits(t.mono_hashes.0, t.mono_hashes.1),
        if par_ok {
            "OK (bit-idéntico)"
        } else {
            "FALLÓ"
        },
        if simd_ok {
            "OK (bit-idéntico)"
        } else {
            "distinto"
        },
    );
}

/// spade's CDT and refinement over a concave piece, and its reproducibility.
fn run_mesh() {
    use toile_mesh::cdt;
    const MAX_AREA: f64 = 2.0e-5;
    let contour = toile_engine::demo::bodice_contour();

    let t = Instant::now();
    let mesh = cdt::triangulate(&contour, MAX_AREA).expect("the demo contour is finite");
    let ms1 = t.elapsed().as_secs_f64() * 1000.0;

    let t = Instant::now();
    let mesh2 = cdt::triangulate(&contour, MAX_AREA).expect("the demo contour is finite");
    let ms2 = t.elapsed().as_secs_f64() * 1000.0;

    let (h1, h2) = (cdt::mesh_hash(&mesh), cdt::mesh_hash(&mesh2));
    println!(
        "\n── spade CDT+refinement · contorno {} pts · pieza cóncava ──",
        contour.len()
    );
    println!(
        "malla            {} vértices · {} triángulos",
        mesh.vertices.len(),
        mesh.triangles.len() / 3
    );
    println!("triangulación    {ms1:7.3} ms (2ª corrida: {ms2:.3} ms)");
    println!(
        "hash             {h1:#018x}  reproducibilidad: {}",
        same_bits(h1, h2)
    );
}
