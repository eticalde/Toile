mod mesh;
mod sheet;

use std::time::Instant;

use toile_sim::xpbd::{self, Seams, Stage};

use self::sheet::{Scene, build};
use super::scene::{DT, same_bits};

const WARMUP: usize = 30;
const TIMED: usize = 240;

/// One substep of the reference scalar path over the benchmark scene.
fn step(s: &mut Scene, seams: &Seams) {
    let (state, cons, sdf) = (&mut s.state, &s.cons, &s.sdf);
    xpbd::substep(state, cons, seams, &Stage::around(sdf), None, DT);
}

/// What the coloured paths answer when a scene carries a pass they lack.
///
/// The benchmark sheet hangs in the void with nothing sewn, nothing under it
/// and nothing holding it, which is the only kind of scene they solve whole,
/// so they never answer it here.
const WHOLE: &str = "the benchmark sheet hangs in the void";

/// Milliseconds per timed substep, and the final position hash.
fn measure(target: usize, mut one: impl FnMut(&mut Scene)) -> (f64, u64) {
    let mut scene = build(target);
    for _ in 0..WARMUP {
        one(&mut scene);
    }
    let t = Instant::now();
    for _ in 0..TIMED {
        one(&mut scene);
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
    mesh::run();
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
    let (mono, h1) = measure(target, |s| step(s, &no_seams));
    let (_, h2) = measure(target, |s| step(s, &no_seams));

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
            let stage = Stage::around(&s.sdf);
            pool.install(|| {
                xpbd::substep_colored(&mut s.state, colored, &no_seams, &stage, None, DT)
                    .expect(WHOLE);
            });
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
            let stage = Stage::around(&s.sdf);
            pool.install(|| {
                xpbd::substep_colored_simd(&mut s.state, colored, &no_seams, &stage, None, DT)
                    .expect(WHOLE);
            });
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
    let mut s = build(target);
    let mut normals = vec![0.0f32; s.state.len() * 3];
    for _ in 0..WARMUP {
        step(&mut s, &no_seams);
    }
    let t = Instant::now();
    for _ in 0..60 {
        xpbd::vertex_normals(&s.state, &s.tris, &mut normals);
    }
    t.elapsed().as_secs_f64() * 1000.0 / 60.0
}

fn run_size(target: usize) {
    let probe = build(target);
    let n = probe.state.len();
    let colored = xpbd::color_constraints(&probe.cons, n)
        .expect("the benchmark sheet is plain stretch, with no elastic and no limit");
    println!(
        "\n── {} vértices · {} constraints · {} triángulos · {} colores ──",
        n,
        probe.cons.len(),
        probe.tris.len() / 3,
        colored.colors()
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
        "determinismo     secuencial: {} · paralelo: {} · simd vs coloreado: {}",
        same_bits(t.mono_hashes.0, t.mono_hashes.1),
        if par_ok {
            "OK (bit-idéntico)"
        } else {
            "FALLÓ"
        },
        if simd_ok {
            "OK (bit-idéntico)"
        } else {
            "FALLÓ"
        },
    );
    // Not a verdict: the coloured paths sweep a permuted set colour by colour
    // and the scalar path sweeps the original in its stored order, so the two
    // are different arithmetic on purpose. The line exists because the label it
    // replaces claimed a comparison the bench never made.
    println!(
        "barrido          coloreado vs escalar: {} — el coloreado permuta el set y barre por color",
        if t.colored[0].2 == t.mono_hashes.0 {
            "mismos bits"
        } else {
            "bits distintos"
        }
    );
}
