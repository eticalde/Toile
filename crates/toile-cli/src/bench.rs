//! `toile bench` — Spike 1 (issue #33): benchmark del kernel XPBD con el
//! patrón de acceso de una malla real.
//!
//! La grilla se construye ordenada y después se baraja la numeración de
//! vértices y el orden de las constraints con un LCG de semilla fija: mismo
//! gather/scatter que deja una CDT, y 100% reproducible.

use std::time::Instant;
use toile_sim::xpbd::{self, DistanceConstraints, SdfGrid, Seams, State};

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
        strain_limit: 0.0,
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

    let sdf = SdfGrid::sphere(256, 1.4 / 255.0, [-0.7, -0.7, -0.7], [0.0, 0.0, 0.0], 0.15);

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
    let no_seams = Seams::default();
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
        xpbd::substep(&mut s.state, &s.cons, &no_seams, &s.sdf, DT);
    });
    let (_, hash_mono2) = measure(target, |s| {
        xpbd::substep(&mut s.state, &s.cons, &no_seams, &s.sdf, DT);
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
        xpbd::substep(&mut scene.state, &scene.cons, &no_seams, &scene.sdf, DT);
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
    if args.iter().any(|a| a == "--seams") {
        run_seams();
        return;
    }
    if args.iter().any(|a| a == "--incr-async") {
        run_incremental_async();
        return;
    }
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

fn bodice_contour() -> Vec<[f64; 2]> {
    toile_engine::couture::demo_bodice_contour()
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
    let no_seams = Seams::default();
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
        let sdf = SdfGrid::sphere(256, 1.4 / 255.0, [-0.7, -0.7, -0.7], [0.0, 0.0, 0.0], 0.15);

        // Drapeado inicial: 1 s de tiempo de sim.
        for _ in 0..600 {
            xpbd::substep(&mut state, &cons, &no_seams, &sdf, DT);
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
                xpbd::substep(&mut state, &cons, &no_seams, &sdf, DT);
            }
        }

        // Re-convergencia tras soltar: energía RMS bajo umbral sostenido.
        let mut steps = 0usize;
        let mut prev_e = f32::MAX;
        let mut rising = false;
        let mut quiet = 0u32;
        let inv_n = 1.0 / n as f32;
        // Mismo criterio de sueño que el hilo de sim (toile-engine::sync):
        // energía al final de cada tick de 10 substeps, 3 ticks quietos.
        while quiet < 3 && steps < 6000 {
            xpbd::substep(&mut state, &cons, &no_seams, &sdf, DT);
            steps += 1;
            let e = xpbd::kinetic_energy(&state);
            if e > prev_e {
                rising = true;
            } else if rising {
                xpbd::zero_velocities(&mut state);
                rising = false;
            }
            prev_e = e;
            if steps.is_multiple_of(10) {
                if e * inv_n < 2.0e-6 {
                    quiet += 1;
                } else {
                    quiet = 0;
                }
            }
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

/// Spike 2 — pipeline asíncrono real: hilo de sim + buzón latest-wins +
/// snapshots arc-swap. Mide la latencia comando → primer snapshot que ya
/// incorpora la edición, bajo storm a 60 Hz reales.
fn run_incremental_async() {
    use std::time::Duration;
    use toile_doc::model::{Command, Doc, Piece};
    use toile_engine::{couture::ShapePipeline, sync};

    let mut doc = Doc {
        pieces: vec![Piece {
            contour: bodice_contour(),
        }],
    };
    let mut pipe = ShapePipeline::build(&doc.pieces[0].contour, 256, 2.0e-5);
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
    let cons = pipe.constraints(1.0e-8);
    let n_edges = cons.len();
    let sdf = SdfGrid::sphere(256, 1.4 / 255.0, [-0.7, -0.7, -0.7], [0.0, 0.0, 0.0], 0.15);

    let handle = sync::spawn(state, cons, sdf, DT, 10);

    // Drapeado inicial: esperar a que la sim converja y se duerma.
    let t0 = Instant::now();
    while !handle.snapshot().converged && t0.elapsed() < Duration::from_secs(15) {
        std::thread::sleep(Duration::from_millis(1));
    }
    let initial = t0.elapsed().as_secs_f64();

    // Storm a 60 Hz reales: derivar en este hilo, mandar, esperar el
    // primer snapshot con la generación aplicada.
    const POINT: usize = 68;
    let base = doc.pieces[0].contour[POINT];
    let frame_dur = Duration::from_micros(16_667);
    let mut latency_ms = Vec::with_capacity(120);
    let mut derive_ms = Vec::with_capacity(120);
    for f in 0..120u32 {
        let frame_start = Instant::now();
        let t = f64::from(f) / 120.0;
        let osc = 0.03 * (t * std::f64::consts::TAU * 2.0).sin();
        Command::MovePoint {
            piece: 0,
            point: POINT,
            to: [base[0] + osc, base[1] + osc * 0.5],
        }
        .apply(&mut doc);
        let td = Instant::now();
        let rests = pipe.derive(&doc.pieces[0].contour);
        derive_ms.push(td.elapsed().as_secs_f64() * 1000.0);
        let generation = u64::from(f) + 1;
        let sent = Instant::now();
        handle.send_rests(generation, rests.to_vec());
        loop {
            if handle.snapshot().generation >= generation {
                latency_ms.push(sent.elapsed().as_secs_f64() * 1000.0);
                break;
            }
            if sent.elapsed() > Duration::from_millis(500) {
                latency_ms.push(sent.elapsed().as_secs_f64() * 1000.0);
                break;
            }
            std::thread::sleep(Duration::from_micros(100));
        }
        let spent = frame_start.elapsed();
        if spent < frame_dur {
            std::thread::sleep(frame_dur - spent);
        }
    }

    // Re-convergencia (reloj de pared) tras soltar.
    let t0 = Instant::now();
    while !handle.snapshot().converged && t0.elapsed() < Duration::from_secs(15) {
        std::thread::sleep(Duration::from_millis(1));
    }
    let reconv = t0.elapsed().as_secs_f64();
    let snap = handle.snapshot();
    let nan_free = snap.positions.iter().all(|x| x.is_finite());
    handle.stop();

    let avg = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    let max = |v: &[f64]| v.iter().cloned().fold(0.0f64, f64::max);
    println!("\n── spike 2 · pipeline asíncrono · storm a 60 Hz reales ──");
    println!(
        "malla            {n} vértices · {n_edges} aristas · sim en hilo propio (10 substeps/tick)"
    );
    println!("drapeado inicial {initial:7.2} s de reloj hasta dormir  (presupuesto: <10 s)");
    println!(
        "derive (hilo UI) {:7.3} ms promedio · {:.3} ms máximo",
        avg(&derive_ms),
        max(&derive_ms)
    );
    println!(
        "latencia edición {:7.3} ms promedio · {:.3} ms máximo  (presupuesto: <200 ms)",
        avg(&latency_ms),
        max(&latency_ms)
    );
    println!("re-convergencia  {reconv:7.2} s de reloj tras soltar  (presupuesto: 2–3 s)");
    println!(
        "estado final     {} · sim dormida: {}",
        if nan_free { "sin NaN" } else { "NaN!" },
        if handle_converged(&snap) {
            "sí (0% CPU)"
        } else {
            "no"
        }
    );
}

fn handle_converged(s: &toile_engine::sync::Snapshot) -> bool {
    s.converged
}

/// Rectángulo CCW muestreado cada ~step; devuelve el contorno y las
/// fracciones de las 4 esquinas: [fin_inferior, fin_derecho, fin_superior, 1].
fn rect_contour(w: f64, h: f64, step: f64) -> (Vec<[f64; 2]>, [f64; 4]) {
    let per = 2.0 * (w + h);
    let mut pts = Vec::new();
    let line = |pts: &mut Vec<[f64; 2]>, a: [f64; 2], b: [f64; 2]| {
        let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        let n = (len / step).ceil() as usize;
        for i in 0..n {
            let t = i as f64 / n as f64;
            pts.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
        }
    };
    line(&mut pts, [0.0, 0.0], [w, 0.0]);
    line(&mut pts, [w, 0.0], [w, h]);
    line(&mut pts, [w, h], [0.0, h]);
    line(&mut pts, [0.0, h], [0.0, 0.0]);
    (pts, [w / per, (w + h) / per, (2.0 * w + h) / per, 1.0])
}

/// Spike 3 — dos piezas con largos de costura distintos (10% embebido),
/// cosidas por ambos costados alrededor de la esfera.
fn run_seams() {
    use toile_engine::couture::{self, ShapePipeline};

    const H_FRONT: f64 = 0.55;
    const H_BACK: f64 = 0.50; // 10% más corto: embebido en el costado
    const W: f64 = 0.46;
    const MAX_AREA: f64 = 4.0e-5;
    const RAMP_STEPS: usize = 450;
    const SEAM_CAP: f32 = 0.002;

    // (perturbación_x, perturbación_z) por corrida de estabilidad.
    let build_and_drape =
        |dx: f32, dz: f32, hot_edit: bool| -> (f64, f64, f64, f32, f32, [f32; 3], u64) {
            let (front_c, ff) = rect_contour(W, H_FRONT, 0.01);
            let (back_c, fb) = rect_contour(W, H_BACK, 0.01);
            let mut front = ShapePipeline::build(&front_c, 192, MAX_AREA);
            let back = ShapePipeline::build(&back_c, 192, MAX_AREA);
            let (na, nb) = (front.pos2d.len(), back.pos2d.len());
            let n = na + nb;

            // Posicionamiento tipo poncho: ambas piezas horizontales sobre
            // la esfera, con los bordes de hombro adyacentes sobre el polo
            // (la gravedad hace el arreglo, estable como en el spike 2) y
            // el cosido progresivo cierra hombro y costados al caer.
            let mut state = State::new(n);
            for i in 0..na {
                state.px[i] = (front.pos2d[i][0] - W * 0.5) as f32 + dx;
                state.py[i] = 0.32;
                state.pz[i] = (0.005 + front.pos2d[i][1]) as f32 + dz;
            }
            for i in 0..nb {
                state.px[na + i] = (back.pos2d[i][0] - W * 0.5) as f32 + dx;
                state.py[na + i] = 0.32;
                state.pz[na + i] = (-0.005 - back.pos2d[i][1]) as f32 + dz;
            }

            let ca = front.constraints(1.0e-8);
            let cb = back.constraints(1.0e-8);
            let n_edges_front = ca.len();
            let mut cons = DistanceConstraints {
                a: ca
                    .a
                    .iter()
                    .chain(
                        cb.a.iter()
                            .map(|v| *v + na as u32)
                            .collect::<Vec<_>>()
                            .iter(),
                    )
                    .copied()
                    .collect(),
                b: ca
                    .b
                    .iter()
                    .chain(
                        cb.b.iter()
                            .map(|v| *v + na as u32)
                            .collect::<Vec<_>>()
                            .iter(),
                    )
                    .copied()
                    .collect(),
                rest: ca.rest.iter().chain(cb.rest.iter()).copied().collect(),
                compliance: ca
                    .compliance
                    .iter()
                    .chain(cb.compliance.iter())
                    .copied()
                    .collect(),
                strain_limit: 1.02,
            };

            // Hombro (bordes inferiores adyacentes sobre el polo) + costados
            // derecho e izquierdo (largos distintos: el embebido del 10% lo
            // absorbe el emparejamiento por fracciones relativas).
            let (mut sa, mut sb) =
                couture::pair_seam(&front, (0.0, ff[0]), &back, (0.0, fb[0]), na as u32, 40);
            let (ra, rb) =
                couture::pair_seam(&front, (ff[0], ff[1]), &back, (fb[0], fb[1]), na as u32, 60);
            let (la, lb) =
                couture::pair_seam(&front, (ff[2], ff[3]), &back, (fb[2], fb[3]), na as u32, 60);
            sa.extend(ra);
            sb.extend(rb);
            sa.extend(la);
            sb.extend(lb);
            let n_pairs = sa.len();
            let mut seams = Seams {
                a: sa,
                b: sb,
                compliance: 1.0e-5,
                max_step: SEAM_CAP,
                iterations: 4,
            };
            let sdf = SdfGrid::sphere(256, 1.4 / 255.0, [-0.7, -0.7, -0.7], [0.0, 0.0, 0.0], 0.15);

            let t_drape = Instant::now();
            // Drapeado inicial con cosido progresivo (rampa exponencial de
            // compliance) y criterio de sueño sostenido.
            let inv_n = 1.0 / n as f32;
            let mut prev_e = f32::MAX;
            let mut rising = false;
            let mut quiet = 0u32;
            let mut steps = 0usize;
            while quiet < 3 && steps < 12000 {
                if steps < RAMP_STEPS {
                    let t = steps as f32 / RAMP_STEPS as f32;
                    seams.compliance = 1.0e-5 * (1.0e-9f32 / 1.0e-5).powf(t);
                } else {
                    seams.compliance = 1.0e-9;
                    seams.max_step = 0.01;
                }
                xpbd::substep(&mut state, &cons, &seams, &sdf, DT);
                steps += 1;
                let e = xpbd::kinetic_energy(&state);
                if e > prev_e {
                    rising = true;
                } else if rising {
                    xpbd::zero_velocities(&mut state);
                    rising = false;
                }
                prev_e = e;
                if steps.is_multiple_of(10) {
                    if e * inv_n < 2.0e-6 {
                        quiet += 1;
                    } else {
                        quiet = 0;
                    }
                }
            }
            let drape_s = steps as f64 / 600.0;
            let drape_wall = t_drape.elapsed().as_secs_f64();

            // Edición en caliente de un borde cosido: ensanchar el frente 3 cm
            // por lado en la base y re-drapear.
            let mut edit_s = 0.0;
            if hot_edit {
                let mut edited = front_c.clone();
                for p in edited.iter_mut() {
                    if p[1] < 1.0e-9 {
                        // borde inferior: estirar en x alrededor del centro
                        p[0] = W * 0.5 + (p[0] - W * 0.5) * (1.0 + 0.06 / W);
                    }
                }
                cons.rest[..n_edges_front].copy_from_slice(&front.derive(&edited)[..n_edges_front]);
                let mut quiet = 0u32;
                let mut steps = 0usize;
                prev_e = f32::MAX;
                rising = false;
                while quiet < 3 && steps < 9000 {
                    xpbd::substep(&mut state, &cons, &seams, &sdf, DT);
                    steps += 1;
                    let e = xpbd::kinetic_energy(&state);
                    if e > prev_e {
                        rising = true;
                    } else if rising {
                        xpbd::zero_velocities(&mut state);
                        rising = false;
                    }
                    prev_e = e;
                    if steps.is_multiple_of(10) {
                        if e * inv_n < 2.0e-6 {
                            quiet += 1;
                        } else {
                            quiet = 0;
                        }
                    }
                }
                edit_s = steps as f64 / 600.0;
            }

            // Métricas de costura: separación entre pares.
            let (mut gap_max, mut gap_sum) = (0.0f32, 0.0f32);
            for k in 0..n_pairs {
                let (ia, ib) = (seams.a[k] as usize, seams.b[k] as usize);
                let g = ((state.px[ib] - state.px[ia]).powi(2)
                    + (state.py[ib] - state.py[ia]).powi(2)
                    + (state.pz[ib] - state.pz[ia]).powi(2))
                .sqrt();
                gap_max = gap_max.max(g);
                gap_sum += g;
            }
            let mut com = [0.0f32; 3];
            for i in 0..n {
                com[0] += state.px[i];
                com[1] += state.py[i];
                com[2] += state.pz[i];
            }
            for c in com.iter_mut() {
                *c *= inv_n;
            }
            (
                drape_s,
                drape_wall,
                edit_s,
                gap_max,
                gap_sum / n_pairs as f32,
                com,
                xpbd::position_hash(&state),
            )
        };

    println!("\n── spike 3 · dos piezas cosidas · 10% embebido en costados ──");
    let (drape_s, drape_wall, edit_s, gap_max, gap_avg, com, h1) = build_and_drape(0.0, 0.0, true);
    println!(
        "drapeado inicial {drape_s:7.2} s de sim · {drape_wall:.2} s de pared (batch sin pacing)  (presupuesto: <10 s de pared)"
    );
    println!(
        "costuras         gap máx {:.2} mm · prom {:.2} mm  (cosida ⇒ ~espaciado de malla)",
        gap_max * 1000.0,
        gap_avg * 1000.0
    );
    println!("edición cosida   {edit_s:7.2} s de sim para re-converger tras +6 cm de base");

    // Estabilidad Sensitive Couture: perturbar la posición inicial y
    // comparar el equilibrio alcanzado.
    let runs = [(0.005f32, -0.004f32), (-0.005, 0.004)];
    let mut com_spread = 0.0f32;
    let mut gap_worst = gap_max;
    for (dx, dz) in runs {
        let (_, _, _, g, _, c, _) = build_and_drape(dx, dz, false);
        gap_worst = gap_worst.max(g);
        let d =
            ((c[0] - com[0]).powi(2) + (c[1] - com[1]).powi(2) + (c[2] - com[2]).powi(2)).sqrt();
        com_spread = com_spread.max(d);
    }
    println!(
        "estabilidad      3 posiciones iniciales → centro de masa dentro de {:.1} mm · peor gap {:.2} mm",
        com_spread * 1000.0,
        gap_worst * 1000.0
    );

    let (_, _, _, _, _, _, h2) = build_and_drape(0.0, 0.0, true);
    println!(
        "determinismo     {}",
        if h1 == h2 {
            "OK (corrida completa bit-idéntica)"
        } else {
            "FALLÓ"
        }
    );
}
