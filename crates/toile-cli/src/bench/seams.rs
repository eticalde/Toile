use std::time::Instant;

use toile_engine::couture::{ShapePipeline, pair_seam};
use toile_engine::demo;
use toile_sim::xpbd::{DistanceConstraints, SdfGrid, Seams, State, position_hash};

use super::scene::{same_bits, seconds, settle, settle_with};

const W: f64 = 0.46;
const H_FRONT: f64 = 0.55;
/// Ten per cent shorter than the front: the ease the side seams must absorb.
const H_BACK: f64 = 0.50;
const SAMPLES: usize = 192;
const MAX_AREA: f64 = 4.0e-5;

/// Substeps over which seam compliance ramps from soft to firm.
const RAMP_STEPS: usize = 450;
const SEAM_START: f32 = 1.0e-5;
const SEAM_FIRM: f32 = 1.0e-9;
const SEAM_CAP: f32 = 0.002;

/// A CCW rectangle sampled every ~`step`, with the arc fractions of its four
/// corners.
fn rect_contour(w: f64, h: f64, step: f64) -> (Vec<[f64; 2]>, [f64; 4]) {
    let per = 2.0 * (w + h);
    let mut pts = Vec::new();
    let mut line = |a: [f64; 2], b: [f64; 2]| {
        let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        let n = (len / step).ceil() as usize;
        for i in 0..n {
            let t = i as f64 / n as f64;
            pts.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
        }
    };
    line([0.0, 0.0], [w, 0.0]);
    line([w, 0.0], [w, h]);
    line([w, h], [0.0, h]);
    line([0.0, h], [0.0, 0.0]);
    (pts, [w / per, (w + h) / per, (2.0 * w + h) / per, 1.0])
}

/// Two panels sewn front to back, positioned like a poncho over the avatar.
struct Garment {
    front: ShapePipeline,
    front_contour: Vec<[f64; 2]>,
    n_front_edges: usize,
    state: State,
    cons: DistanceConstraints,
    seams: Seams,
    sdf: SdfGrid,
}

/// Sews shoulder and both sides, leaving the centre 30% of the shoulder open.
///
/// That gap is the neckline, and it is what holds the garment up: the pole of
/// the avatar comes through it and the piece locks on geometrically, exactly
/// as real clothing does when the neckline is smaller than the body below it.
fn sew(front: &ShapePipeline, back: &ShapePipeline, ff: [f64; 4], fb: [f64; 4], na: u32) -> Seams {
    let (mut a, mut b) = pair_seam(
        front,
        (0.0, ff[0] * 0.35),
        back,
        (0.0, fb[0] * 0.35),
        na,
        15,
    );
    let (ha, hb) = pair_seam(
        front,
        (ff[0] * 0.65, ff[0]),
        back,
        (fb[0] * 0.65, fb[0]),
        na,
        15,
    );
    a.extend(ha);
    b.extend(hb);
    let (ra, rb) = pair_seam(front, (ff[0], ff[1]), back, (fb[0], fb[1]), na, 60);
    let (la, lb) = pair_seam(front, (ff[2], ff[3]), back, (fb[2], fb[3]), na, 60);
    a.extend(ra);
    b.extend(rb);
    a.extend(la);
    b.extend(lb);
    Seams {
        a,
        b,
        compliance: SEAM_START,
        max_step: SEAM_CAP,
        iterations: 4,
    }
}

/// Concatenates two pieces' constraints into one combined solver set.
fn combine(front: &ShapePipeline, back: &ShapePipeline, na: u32) -> DistanceConstraints {
    let (ca, cb) = (front.constraints(1.0e-8), back.constraints(1.0e-8));
    DistanceConstraints {
        a: ca
            .a
            .iter()
            .copied()
            .chain(cb.a.iter().map(|v| v + na))
            .collect(),
        b: ca
            .b
            .iter()
            .copied()
            .chain(cb.b.iter().map(|v| v + na))
            .collect(),
        rest: ca.rest.iter().chain(cb.rest.iter()).copied().collect(),
        compliance: ca
            .compliance
            .iter()
            .chain(cb.compliance.iter())
            .copied()
            .collect(),
        strain_limit: 1.03,
        strain_sweeps: 4,
    }
}

/// Builds the garment with its starting position offset by `(dx, dz)`, which
/// is how the stability run perturbs it.
fn assemble(dx: f32, dz: f32) -> Garment {
    let (front_contour, ff) = rect_contour(W, H_FRONT, 0.01);
    let (back_contour, fb) = rect_contour(W, H_BACK, 0.01);
    let front = ShapePipeline::build(&front_contour, SAMPLES, MAX_AREA);
    let back = ShapePipeline::build(&back_contour, SAMPLES, MAX_AREA);
    let (na, nb) = (front.pos2d.len(), back.pos2d.len());

    let mut state = State::new(na + nb);
    for i in 0..na {
        state.px[i] = (front.pos2d[i][0] - W * 0.5) as f32 + dx;
        state.py[i] = 0.16;
        state.pz[i] = (0.005 + front.pos2d[i][1]) as f32 + dz;
    }
    for i in 0..nb {
        state.px[na + i] = (back.pos2d[i][0] - W * 0.5) as f32 + dx;
        state.py[na + i] = 0.16;
        state.pz[na + i] = (-0.005 - back.pos2d[i][1]) as f32 + dz;
    }

    let cons = combine(&front, &back, na as u32);
    let seams = sew(&front, &back, ff, fb, na as u32);
    Garment {
        n_front_edges: front.constraints(1.0e-8).len(),
        front,
        front_contour,
        state,
        cons,
        seams,
        sdf: demo::avatar_sdf(),
    }
}

/// Drapes with progressive sewing: seam compliance ramps exponentially from
/// soft to firm, so the first frames cannot generate extreme forces.
fn drape(g: &mut Garment) -> (f64, f64) {
    let wall = Instant::now();
    let steps = settle_with(
        &mut g.state,
        &g.cons,
        &mut g.seams,
        &g.sdf,
        12_000,
        |step, seams| {
            if step < RAMP_STEPS {
                let t = step as f32 / RAMP_STEPS as f32;
                seams.compliance = SEAM_START * (SEAM_FIRM / SEAM_START).powf(t);
            } else {
                seams.compliance = SEAM_FIRM;
                seams.max_step = 0.01;
            }
        },
    );
    (seconds(steps), wall.elapsed().as_secs_f64())
}

/// Widens the front panel's hem by 6 cm and re-drapes, editing a sewn edge
/// while the garment is on the avatar.
fn hot_edit(g: &mut Garment) -> f64 {
    let mut edited = g.front_contour.clone();
    for p in &mut edited {
        if p[1] < 1.0e-9 {
            p[0] = W * 0.5 + (p[0] - W * 0.5) * (1.0 + 0.06 / W);
        }
    }
    let rests = g.front.derive(&edited);
    g.cons.rest[..g.n_front_edges].copy_from_slice(&rests[..g.n_front_edges]);
    seconds(settle(&mut g.state, &g.cons, &g.seams, &g.sdf, 9_000))
}

/// Largest and mean separation between sewn pairs.
fn seam_gaps(g: &Garment) -> (f32, f32) {
    let (mut worst, mut sum) = (0.0f32, 0.0f32);
    for k in 0..g.seams.len() {
        let (ia, ib) = (g.seams.a[k] as usize, g.seams.b[k] as usize);
        let gap = ((g.state.px[ib] - g.state.px[ia]).powi(2)
            + (g.state.py[ib] - g.state.py[ia]).powi(2)
            + (g.state.pz[ib] - g.state.pz[ia]).powi(2))
        .sqrt();
        worst = worst.max(gap);
        sum += gap;
    }
    (worst, sum / g.seams.len() as f32)
}

fn centre_of_mass(g: &Garment) -> [f32; 3] {
    let inv_n = 1.0 / g.state.len() as f32;
    let mut com = [0.0f32; 3];
    for i in 0..g.state.len() {
        com[0] += g.state.px[i];
        com[1] += g.state.py[i];
        com[2] += g.state.pz[i];
    }
    for c in &mut com {
        *c *= inv_n;
    }
    com
}

pub fn run() {
    println!("\n── dos piezas cosidas · 10% embebido en costados ──");
    let mut g = assemble(0.0, 0.0);
    let (sim_s, wall_s) = drape(&mut g);
    let (gap_max, gap_avg) = seam_gaps(&g);
    let com = centre_of_mass(&g);
    let edit_s = hot_edit(&mut g);
    let hash = position_hash(&g.state);

    println!(
        "drapeado inicial {sim_s:7.2} s de sim · {wall_s:.2} s de pared (batch sin pacing)  (presupuesto: <10 s de pared)"
    );
    println!(
        "costuras         gap máx {:.2} mm · prom {:.2} mm  (cosida ⇒ ~espaciado de malla)",
        gap_max * 1000.0,
        gap_avg * 1000.0
    );
    println!("edición cosida   {edit_s:7.2} s de sim para re-converger tras +6 cm de base");

    // Sensitive Couture's stability test: perturb the starting position and
    // check the garment reaches the same equilibrium.
    let mut spread = 0.0f32;
    let mut worst_gap = gap_max;
    for (dx, dz) in [(0.005f32, -0.004f32), (-0.005, 0.004)] {
        let mut p = assemble(dx, dz);
        drape(&mut p);
        let (g2, _) = seam_gaps(&p);
        worst_gap = worst_gap.max(g2);
        let c = centre_of_mass(&p);
        let d =
            ((c[0] - com[0]).powi(2) + (c[1] - com[1]).powi(2) + (c[2] - com[2]).powi(2)).sqrt();
        spread = spread.max(d);
    }
    println!(
        "estabilidad      3 posiciones iniciales → centro de masa dentro de {:.1} mm · peor gap {:.2} mm",
        spread * 1000.0,
        worst_gap * 1000.0
    );

    let mut again = assemble(0.0, 0.0);
    drape(&mut again);
    hot_edit(&mut again);
    println!(
        "determinismo     {}",
        same_bits(hash, position_hash(&again.state))
    );
}
