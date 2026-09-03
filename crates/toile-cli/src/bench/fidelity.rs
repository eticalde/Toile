use toile_sim::xpbd::{self, DistanceConstraints, SdfGrid, Seams, State};

use super::scene::{DT, settle};

const W: usize = 81; // 0.40 m at 5 mm
const H: usize = 121; // 0.60 m at 5 mm
const S: f32 = 0.005;
const PATTERN_LEN: f32 = (H as f32 - 1.0) * S;

/// Sartorial tolerance: a pattern measurement may not drift by more than this.
const TOL_PCT: f64 = 1.0;

/// Extra simulated seconds without kinetic damping, to tell material stretch
/// from a shortfall of iterations.
const REFINE_SUBSTEPS: usize = 6000;

/// A fabric is the whole tuple, never one value on its own: compliances only
/// mean something relative to the mass they act on.
struct Fabric {
    name: &'static str,
    /// Kilograms per particle. `1.0` is the legacy placeholder; a real fabric
    /// is density × area per particle.
    mass: f32,
    strain_limit: f32,
    /// Structural, shear and bending compliance.
    compliance: [f32; 3],
}

fn fabrics() -> [Fabric; 4] {
    // 200 g/m² medium cotton over one 5 mm cell.
    let real_mass = 0.2 * S * S;
    [
        Fabric {
            name: "masa 1 kg · legacy  · sin lím",
            mass: 1.0,
            strain_limit: 0.0,
            compliance: [1.0e-8, 5.0e-7, 1.0e-5],
        },
        Fabric {
            name: "masa 1 kg · legacy  · 1.005 ",
            mass: 1.0,
            strain_limit: 1.005,
            compliance: [1.0e-8, 5.0e-7, 1.0e-5],
        },
        Fabric {
            name: "algodón físico      · sin lím",
            mass: real_mass,
            strain_limit: 0.0,
            compliance: [1.0e-4, 1.0e-2, 10.0],
        },
        Fabric {
            name: "algodón físico      · 1.005 ",
            mass: real_mass,
            strain_limit: 1.005,
            compliance: [1.0e-4, 1.0e-2, 10.0],
        },
    ]
}

/// A banner of known dimensions, pinned along its top edge.
fn banner(f: &Fabric) -> (State, DistanceConstraints) {
    let n = W * H;
    let mut state = State::new(n);
    for j in 0..H {
        for i in 0..W {
            let v = j * W + i;
            state.px[v] = i as f32 * S - 0.2;
            state.py[v] = -(j as f32) * S;
            // A microscopic out-of-plane jitter. Without it the lattice is
            // trapped at exactly z=0 and compression cannot buckle — real
            // cloth buckles instantly, and the in-plane zigzag mode diverges.
            state.pz[v] = ((i * 7 + j * 13) % 17) as f32 * 1.0e-5;
            state.inv_mass[v] = if j == 0 { 0.0 } else { 1.0 / f.mass };
        }
    }

    let mut cons = DistanceConstraints {
        strain_limit: f.strain_limit,
        strain_sweeps: 16,
        ..Default::default()
    };
    let push = |a: usize, b: usize, c: f32, cons: &mut DistanceConstraints| {
        cons.a.push(a as u32);
        cons.b.push(b as u32);
        let (dx, dy) = (state.px[b] - state.px[a], state.py[b] - state.py[a]);
        cons.rest.push((dx * dx + dy * dy).sqrt());
        cons.compliance.push(c);
    };
    for j in 0..H {
        for i in 0..W {
            let v = j * W + i;
            if i + 1 < W {
                push(v, v + 1, f.compliance[0], &mut cons);
            }
            if j + 1 < H {
                push(v, v + W, f.compliance[0], &mut cons);
            }
            if i + 1 < W && j + 1 < H {
                push(v, v + W + 1, f.compliance[1], &mut cons);
                push(v + 1, v + W, f.compliance[1], &mut cons);
            }
            if i + 2 < W {
                push(v, v + 2, f.compliance[2], &mut cons);
            }
            if j + 2 < H {
                push(v, v + 2 * W, f.compliance[2], &mut cons);
            }
        }
    }
    (state, cons)
}

/// Mean and worst column elongation against the flat pattern length, as a
/// percentage.
fn stretch(state: &State) -> (f64, f64) {
    let (mut sum, mut worst) = (0.0f64, 0.0f64);
    for i in 0..W {
        let mut len = 0.0f32;
        for j in 0..H - 1 {
            let (a, b) = (j * W + i, (j + 1) * W + i);
            len += ((state.px[b] - state.px[a]).powi(2)
                + (state.py[b] - state.py[a]).powi(2)
                + (state.pz[b] - state.pz[a]).powi(2))
            .sqrt();
        }
        let pct = f64::from(len / PATTERN_LEN - 1.0) * 100.0;
        sum += pct;
        worst = worst.max(pct);
    }
    (sum / W as f64, worst)
}

pub fn run() {
    println!(
        "\n── banner 0.40×0.60 m colgado · largo de patrón {:.0} mm ──",
        f64::from(PATTERN_LEN) * 1000.0
    );
    let no_seams = Seams::default();
    // Far away: no contact, only the cost of the sample.
    let sdf = SdfGrid::sphere(8, 1.0, [-4.0, -14.0, -4.0], [0.0, -10.0, 0.0], 0.1);

    for f in &fabrics() {
        let (mut state, cons) = banner(f);
        settle(&mut state, &cons, &no_seams, &sdf, 12_000);
        let (avg_settle, max_settle) = stretch(&state);

        for _ in 0..REFINE_SUBSTEPS {
            xpbd::substep(&mut state, &cons, &no_seams, &sdf, DT);
        }
        let (avg_ref, max_ref) = stretch(&state);

        let verdict = if avg_ref.is_finite() {
            if max_ref.abs() <= TOL_PCT {
                "✅"
            } else {
                "❌"
            }
        } else {
            "💥 INESTABLE"
        };
        println!(
            "{}  settle {avg_settle:+7.2}% prom / {max_settle:+7.2}% máx  →  refinado {avg_ref:+6.2}% / {max_ref:+6.2}% máx ({:.1} mm) {verdict}",
            f.name,
            max_ref / 100.0 * f64::from(PATTERN_LEN) * 1000.0
        );
    }
    println!(
        "tolerancia sartorial: ±{TOL_PCT:.0}% · refinado = 10 s de sim extra sin kinetic damping"
    );
}
