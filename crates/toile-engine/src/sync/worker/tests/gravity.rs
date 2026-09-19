use toile_sim::xpbd::{self, Seams, Stage, State};

use super::{Sim, ball, sim_of};
use crate::couture::{self, COMPLIANCE, SEAM_PASSES, SEAM_STEP, ShapePipeline};

/// How far apart the two halves are let go, in metres: the shipped block's own
/// opening, and wide enough that the seams take a while to pull it shut.
const APART: f32 = 0.08;

/// Where the panels hang, in metres. Well above the fixture's ball, so the one
/// thing that moves them vertically is the pull these tests measure.
const HEIGHT: f32 = 0.5;

/// Substeps each scene is watched for: a fifth of the ramp, so a scene still
/// weightless here is weightless because its seams are open and not because
/// the count ran out.
const WATCH: u64 = 90;

/// Simulated seconds per substep, as the engine runs it.
const DT: f32 = 1.0 / 600.0;

/// Substeps the two arms of the measurement are run for: three simulated
/// seconds, the mark the seeded scenes read a worn garment at.
const MARK: u64 = 1800;

/// How far the cloth has to sink before it counts as falling, in metres.
const SANK: f64 = 1.0e-5;

/// A rectangle at the engine's own density.
fn panel() -> ShapePipeline {
    let rectangle = [[0.0, 0.0], [0.30, 0.0], [0.30, 0.20], [0.0, 0.20]];
    let (samples, max_area) = couture::for_contour(&rectangle);
    ShapePipeline::build(&rectangle, samples, max_area).expect("the rectangle is finite")
}

/// Two panels let go side by side `apart` metres from each other, with their
/// hems sewn to each other when `sewn`.
///
/// Both halves come from the same release, so at `apart` of zero the sewn
/// pairs stand in one place and the product is shut before it is let go.
fn halves(pipe: &ShapePipeline, apart: f32, sewn: bool) -> (State, Seams) {
    let mut state = couture::drop_all(&[pipe, pipe], HEIGHT, None);
    let base = pipe.pos2d.len();
    for i in base..state.len() {
        state.px[i] += apart;
    }
    let hem = pipe.boundary_run((0.0, 0.3));
    let seams = if sewn {
        Seams {
            b: hem.iter().map(|&v| v + base as u32).collect(),
            a: hem,
            compliance: couture::sewing_at(0).0,
            max_step: SEAM_STEP,
            iterations: SEAM_PASSES,
        }
    } else {
        Seams::default()
    };
    (state, seams)
}

/// The mean height of the whole product, in metres.
///
/// Summed in double. A thousand `f32` heights of half a metre accumulate tens
/// of microns of rounding on their own, which is the very size of the first
/// substep's fall — read in single, every scene here looks like it hung for a
/// substep before it dropped.
fn height(state: &State) -> f64 {
    state.py.iter().map(|&y| f64::from(y)).sum::<f64>() / state.len() as f64
}

/// The first substep the product sinks on, watching `substeps` of it; `None`
/// while it is still hanging where it was let go.
fn falls_on(sim: &mut Sim, substeps: u64) -> Option<u64> {
    let start = height(&sim.state);
    (1..=substeps).find(|_| {
        sim.tick();
        height(&sim.state) < start - SANK
    })
}

/// The sim thread holds a sewn product weightless until its seams are shut,
/// and lets everything else fall from its first substep.
///
/// The one test that tells the phase from its absence: neutralising the
/// weightless branch in `Sim::tick` makes all three scenes read 1.
#[test]
fn a_sewn_product_hangs_weightless_until_its_seams_are_shut() {
    let pipe = panel();
    let cons = couture::combine_constraints(&[&pipe, &pipe], COMPLIANCE);
    let scene = |apart, sewn| {
        let (state, seams) = halves(&pipe, apart, sewn);
        sim_of(state, cons.clone(), seams, 1)
    };
    let open = falls_on(&mut scene(APART, true), WATCH);
    let shut = falls_on(&mut scene(0.0, true), WATCH);
    let loose = falls_on(&mut scene(APART, false), WATCH);
    println!("open {open:?} · shut {shut:?} · nothing sewn {loose:?}");

    assert_eq!(loose, Some(1), "a product with nothing sewn falls at once");
    let open = open.expect("the seams shut inside the watch and the product fell");
    assert!(
        open > 1,
        "the open product hung while it was pulled together"
    );
    // The gap ended the phase here and not the ramp: at this substep the same
    // product with its seams still wide would be weightless still.
    assert!(
        couture::closing(open, 1.0),
        "the ramp ran out before the seams shut, at substep {open}"
    );
    // And the branch no shipped scene has ever taken: a product whose sewn
    // pairs already stand in one place is shut before it is let go, so the gap
    // ends the phase at the first substep, deep inside the ramp.
    assert_eq!(shut, Some(1), "a product already shut does not wait");
}

/// What the phase buys, measured: the product is sewn where it was let go.
///
/// Two arms of the same scene — the same two panels, the same 8 cm open, the
/// same seams — differing only in whether the criterion is consulted. What it
/// does not buy is a different end state: both read their seams shut at the
/// mark, which is what the shipped block reads too.
#[test]
fn the_phase_buys_a_product_sewn_where_it_was_let_go() {
    let pipe = panel();
    let cons = couture::combine_constraints(&[&pipe, &pipe], COMPLIANCE);
    let field = ball(0.15).sdf;
    let arm = |weightless: bool| {
        let (mut state, mut seams) = halves(&pipe, APART, true);
        let start = height(&state);
        let (mut shut_at, mut sank) = (None, 0.0f64);
        for at in 0..MARK {
            (seams.compliance, seams.max_step) = couture::sewing_at(at);
            let closing = couture::closing(at, xpbd::seam_gap(&state, &seams));
            if !closing && shut_at.is_none() {
                shut_at = Some(at);
                sank = start - height(&state);
            }
            let stage = Stage::around(&field);
            let stage = if weightless && closing {
                stage.weightless()
            } else {
                stage
            };
            xpbd::substep(&mut state, &cons, &seams, &stage, None, DT);
        }
        let shut_at = shut_at.expect("the seams shut inside the mark");
        (shut_at, sank, xpbd::seam_gap(&state, &seams))
    };
    let (held_at, held_sank, held_gap) = arm(true);
    let (fell_at, fell_sank, fell_gap) = arm(false);
    println!(
        "with the phase: shut at {held_at}, sunk {held_sank:.5} m by then, \
         gap {held_gap:.5} m at {MARK}"
    );
    println!("without it: shut at {fell_at}, sunk {fell_sank:.5} m by then, gap {fell_gap:.5} m");

    assert!(
        held_sank.abs() < SANK,
        "the product was sewn where it was let go: it sank {held_sank} m first"
    );
    assert!(
        fell_sank > 100.0 * SANK,
        "and without the phase it was sewn on the way down: {fell_sank} m"
    );
    assert!(
        held_gap < 0.001 && fell_gap < 0.001,
        "both arms read their seams shut at the mark: {held_gap} and {fell_gap}"
    );
}
