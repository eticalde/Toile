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

    // SIMD (wide f32x8) sobre las mismas constraints coloreadas: cada lane
    // ejecuta las mismas operaciones IEEE que el camino escalar, así que
    // debería ser bit-idéntico a coloring.
    let mut simd_runs = Vec::new();
    for t in [1usize, 4] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(t)
            .build()
            .unwrap();
        let (ms, hash) = measure(target, |s| {
            pool.install(|| xpbd::substep_colored_simd(&mut s.state, &colored, &s.sdf, DT));
        });
        simd_runs.push((t, ms, hash));
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
    for &(t, ms, _) in &simd_runs {
        println!(
            "simd f32x8 ×{t:<2}   {ms:7.3} ms/substep  → {:.0} substeps/frame · speedup {:.2}× vs mono",
            frame(ms),
            ms_mono / ms
        );
    }
    println!("normales         {ms_normals:7.3} ms (cadencia visual)");
    let par_ok = colored_runs.windows(2).all(|w| w[0].2 == w[1].2);
    let simd_ok = simd_runs.iter().all(|&(_, _, h)| h == colored_runs[0].2);
    println!(
        "determinismo     secuencial: {} · paralelo: {} · simd vs escalar: {}",
        if hash_mono == hash_mono2 {
            "OK"
        } else {
            "FALLÓ"
        },
        if par_ok {
            "OK (bit-idéntico)"
        } else {
            "FALLÓ"
        },
        if simd_ok {
            "OK (bit-idéntico)"
        } else {
            "distinto"
        }
    );
}

pub fn run(args: &[String]) {
    if args.iter().any(|a| a == "--incr") {
        run_incremental();
        return;
    }
    let sizes: Vec<usize> = if let Some(i) = args.iter().position(|a| a == "--verts") {
        vec![args[i + 1].parse().expect("--verts N")]
    } else {
        vec![20_000, 50_000]
    };
    println!("toile bench — kernel XPBD, acceso barajado (Spike 1)");
    for s in sizes {
        run_size(s);
    }
    run_mesh();
}

/// Contorno tipo delantero de corpiño: recto en costados y hombro, cóncavo
/// en sisa y escote — la concavidad es el caso que importa (ADR §3.4).
fn bodice_contour() -> Vec<[f64; 2]> {
    let mut pts: Vec<[f64; 2]> = Vec::new();
    let line = |pts: &mut Vec<[f64; 2]>, a: [f64; 2], b: [f64; 2], n: usize| {
        for i in 0..n {
            let t = i as f64 / n as f64;
            pts.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
        }
    };
    let quad = |pts: &mut Vec<[f64; 2]>, a: [f64; 2], c: [f64; 2], b: [f64; 2], n: usize| {
        for i in 0..n {
            let t = i as f64 / n as f64;
            let u = 1.0 - t;
            pts.push([
                u * u * a[0] + 2.0 * u * t * c[0] + t * t * b[0],
                u * u * a[1] + 2.0 * u * t * c[1] + t * t * b[1],
            ]);
        }
    };
    line(&mut pts, [0.0, 0.0], [0.50, 0.0], 20); // cintura
    line(&mut pts, [0.50, 0.0], [0.52, 0.45], 18); // costado
    quad(&mut pts, [0.52, 0.45], [0.36, 0.50], [0.38, 0.68], 30); // sisa (cóncava)
    line(&mut pts, [0.38, 0.68], [0.18, 0.72], 10); // hombro
    quad(&mut pts, [0.18, 0.72], [0.16, 0.56], [0.0, 0.60], 26); // escote (cóncavo)
    line(&mut pts, [0.0, 0.60], [0.0, 0.0], 24); // centro frente
    pts
}

fn run_mesh() {
    use toile_mesh::cdt;
    const MAX_AREA: f64 = 2.0e-5; // ~triángulos de 6 mm de lado
    let contour = bodice_contour();

    let t = Instant::now();
    let mesh = cdt::triangulate(&contour, MAX_AREA);
    let ms1 = t.elapsed().as_secs_f64() * 1000.0;
    let h1 = cdt::mesh_hash(&mesh);

    let t = Instant::now();
    let mesh2 = cdt::triangulate(&contour, MAX_AREA);
    let ms2 = t.elapsed().as_secs_f64() * 1000.0;
    let h2 = cdt::mesh_hash(&mesh2);

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
        if h1 == h2 {
            "OK (bit-idéntica)"
        } else {
            "FALLÓ"
        }
    );
}

/// Spike 2 — vía A síncrona: drag storm sobre el pipeline incremental.
fn run_incremental() {
    use toile_doc::model::{Command, Doc, Piece};
    use toile_engine::couture::ShapePipeline;

    let storm = || -> (f64, usize, usize, usize, Vec<f64>, f64, u64) {
        let mut doc = Doc {
            pieces: vec![Piece {
                contour: bodice_contour(),
            }],
        };

        let t0 = Instant::now();
        let mut pipe = ShapePipeline::build(&doc.pieces[0].contour, 256, 2.0e-5);
        let build_ms = t0.elapsed().as_secs_f64() * 1000.0;

        let n = pipe.pos2d.len();
        let (mut cx, mut cy) = (0.0, 0.0);
        for p in &pipe.pos2d {
            cx += p[0];
            cy += p[1];
        }
        cx /= n as f64;
        cy /= n as f64;

        let mut state = State::new(n);
        for i in 0..n {
            state.px[i] = (pipe.pos2d[i][0] - cx) as f32;
            state.py[i] = 0.35;
            state.pz[i] = (pipe.pos2d[i][1] - cy) as f32;
        }
        let mut cons = pipe.constraints(1.0e-8);
        let sdf = SdfGrid::sphere(128, 1.4 / 127.0, [-0.7, -0.7, -0.7], [0.0, 0.0, 0.0], 0.15);

        // Drapeado inicial: 1 s de tiempo de sim.
        for _ in 0..600 {
            xpbd::substep(&mut state, &cons, &sdf, DT);
        }

        // Storm: 120 frames a 60 Hz, el vértice hombro-sisa oscila ±3 cm.
        const POINT: usize = 68;
        let base = doc.pieces[0].contour[POINT];
        let mut derive_ms = Vec::with_capacity(120);
        for f in 0..120u32 {
            let t = f64::from(f) / 120.0;
            let osc = 0.03 * (t * std::f64::consts::TAU * 2.0).sin();
            Command::MovePoint {
                piece: 0,
                point: POINT,
                to: [base[0] + osc, base[1] + osc * 0.5],
            }
            .apply(&mut doc);
            let t0 = Instant::now();
            let rests = pipe.derive(&doc.pieces[0].contour);
            cons.rest.copy_from_slice(rests);
            derive_ms.push(t0.elapsed().as_secs_f64() * 1000.0);
            for _ in 0..10 {
                xpbd::substep(&mut state, &cons, &sdf, DT);
            }
        }

        // Re-convergencia tras soltar: substeps hasta v_max < 1 cm/s.
        let max_speed = |s: &State| -> f32 {
            let mut m = 0.0f32;
            for i in 0..s.len() {
                let v2 = s.vx[i] * s.vx[i] + s.vy[i] * s.vy[i] + s.vz[i] * s.vz[i];
                m = m.max(v2);
            }
            m.sqrt()
        };
        let mut steps = 0usize;
        let mut prev_e = f32::MAX;
        let mut rising = false;
        while max_speed(&state) > 0.01 && steps < 6000 {
            xpbd::substep(&mut state, &cons, &sdf, DT);
            steps += 1;
            let e = xpbd::kinetic_energy(&state);
            if e > prev_e {
                rising = true;
            } else if rising {
                xpbd::zero_velocities(&mut state);
                rising = false;
            }
            prev_e = e;
        }
        let conv_s = steps as f64 / 600.0;

        (
            build_ms,
            pipe.n_boundary(),
            pipe.n_interior(),
            pipe.edges.len(),
            derive_ms,
            conv_s,
            xpbd::position_hash(&state),
        )
    };

    let (build_ms, nb, ni, ne, derive_ms, conv_s, h1) = storm();
    let (_, _, _, _, _, _, h2) = storm();

    let avg = derive_ms.iter().sum::<f64>() / derive_ms.len() as f64;
    let max = derive_ms.iter().cloned().fold(0.0f64, f64::max);
    println!("\n── spike 2 · vía A síncrona · drag storm 120 frames ──");
    println!(
        "malla            {} frontera + {} interior · {} aristas",
        nb, ni, ne
    );
    println!("build (una vez)  {build_ms:7.1} ms (CDT + clasificación + matriz MVC)");
    println!("derive por edit  {avg:7.3} ms promedio · {max:.3} ms máximo  (presupuesto: <5 ms)");
    println!("re-convergencia  {conv_s:7.2} s de sim tras soltar  (presupuesto: 2–3 s)");
    println!(
        "estado           {} · determinismo storm completo: {}",
        if h1 != 0 { "sin NaN" } else { "?" },
        if h1 == h2 {
            "OK (bit-idéntico)"
        } else {
            "FALLÓ"
        }
    );
}
