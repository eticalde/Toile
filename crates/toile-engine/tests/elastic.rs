#![allow(missing_docs, reason = "a test crate publishes no API surface")]

use toile_engine::couture::{self, COMPLIANCE, HOLDS_ITS_RATIO, Held, ShapePipeline};
use toile_sim::xpbd::{self, DistanceConstraints, SdfGrid, Seams, Stage, State};

/// Simulated seconds per substep, as the engine runs it.
const DT: f32 = 1.0 / 600.0;

/// Substeps the panel is given to take its band in: one simulated second,
/// long past the point the hem stops moving.
const STEPS: usize = 600;

/// The stretch the band is held to, as a fraction of the drawn length.
const GATHERED: f64 = 0.85;

/// A rectangle three tenths of a metre across, meshed at the engine's own
/// density, with its hem opening the contour: the stretch `(0.0, 0.3)` is the
/// 0.30 m bottom edge of a contour one metre round.
fn panel() -> ShapePipeline {
    let rectangle = [[0.0, 0.0], [0.30, 0.0], [0.30, 0.20], [0.0, 0.20]];
    let (samples, max_area) = couture::for_contour(&rectangle);
    ShapePipeline::build(&rectangle, samples, max_area).expect("the rectangle is finite")
}

/// A field the panel never reaches: a pea at the origin, in a grid the panel
/// hangs well above.
///
/// The contact solve is not what is being measured here, and a body under the
/// cloth would be the one thing that could hold a band open.
fn nowhere_near() -> SdfGrid {
    SdfGrid::sphere(32, 0.05, [-0.8, -0.8, -0.8], [0.0, 0.0, 0.0], 0.05)
}

/// How long a run of boundary vertices measures in the state, in metres.
fn along(state: &State, run: &[u32]) -> f32 {
    run.windows(2)
        .map(|pair| {
            let (a, b) = (pair[0] as usize, pair[1] as usize);
            let (dx, dy, dz) = (
                state.px[b] - state.px[a],
                state.py[b] - state.py[a],
                state.pz[b] - state.pz[a],
            );
            (dx * dx + dy * dy + dz * dz).sqrt()
        })
        .sum()
}

/// The edges of one stretch, held to `ratio` at `strength`.
fn band(pipe: &ShapePipeline, run: &[u32], ratio: f64, strength: f64) -> Vec<Held> {
    run.windows(2)
        .filter_map(|pair| pipe.edge_index(pair[0], pair[1]))
        .map(|edge| Held {
            edge,
            ratio: ratio as f32,
            compliance: couture::compliance_of(strength),
        })
        .collect()
}

/// Lets the panel go weightless and returns what it settles to.
fn settle(pipe: &ShapePipeline, cons: &DistanceConstraints) -> State {
    let sdf = nowhere_near();
    let stage = Stage::around(&sdf).weightless();
    let seams = Seams::default();
    let mut state = couture::drop_state(pipe, 0.5);
    for _ in 0..STEPS {
        xpbd::substep(&mut state, cons, &seams, &stage, None, DT);
    }
    state
}

/// The hem's length after settling with the band it is given.
fn hem_after(pipe: &ShapePipeline, hem: &[u32], ratio: Option<f64>) -> (f32, State) {
    let mut cons = pipe.constraints(COMPLIANCE);
    if let Some(ratio) = ratio {
        couture::hold(&mut cons, &band(pipe, hem, ratio, HOLDS_ITS_RATIO));
    }
    let state = settle(pipe, &cons);
    (along(&state, hem), state)
}

/// The elastic does what it says: the stretch it holds comes in, a tighter
/// ratio brings it in further, neither goes past what it asked for, and the
/// cloth across the panel is left where it was.
///
/// Weightless and out of reach of any field, so nothing but the elastic is
/// pulling. On a body the reading would be the body's: a band cannot be
/// shorter than the person it is sitting on.
///
/// How far it comes is the mesh's answer, not the band's. One row of boundary
/// edges asking to be shorter meets, at each of its vertices, half a dozen
/// interior edges asking to stay the length they were drawn at, and it is
/// outvoted: a hem asked for fifteen per cent gives up three. That is the
/// honest behaviour of a gather, and what it is worth is measured here rather
/// than assumed.
#[test]
fn a_held_stretch_comes_in_toward_its_ratio_and_the_rest_does_not() {
    let pipe = panel();
    let hem = pipe.boundary_run((0.0, 0.3));
    let far = pipe.boundary_run((0.5, 0.3));

    let (drawn, loose) = hem_after(&pipe, &hem, None);
    let opposite = along(&loose, &far);
    let (gathered, held) = hem_after(&pipe, &hem, Some(GATHERED));
    let (tighter, _) = hem_after(&pipe, &hem, Some(0.50));
    println!(
        "hem {drawn:.4} m drawn · {gathered:.4} m at {GATHERED} · {tighter:.4} m at 0.50 \
         · far edge {opposite:.4} against {:.4}",
        along(&held, &far)
    );

    assert!(
        gathered < drawn,
        "the band came in: {gathered} against {drawn}"
    );
    assert!(
        tighter < gathered,
        "and a tighter ratio brings it further: {tighter} against {gathered}"
    );
    assert!(
        gathered > drawn * GATHERED as f32,
        "neither goes past what it asked for: {gathered}"
    );
    assert!(
        tighter > drawn * 0.50,
        "nor does the tighter one: {tighter}"
    );
    assert!(
        (along(&held, &far) - opposite).abs() < opposite * 0.01,
        "while the edge across the panel is where it was: {} against {opposite}",
        along(&held, &far)
    );
}

/// A product with no elastic runs today's arithmetic: the same constraints,
/// bit for bit, and therefore the same drape.
#[test]
fn a_panel_with_no_elastic_is_the_panel_of_today() {
    let pipe = panel();
    let plain = pipe.constraints(COMPLIANCE);
    let mut cons = pipe.constraints(COMPLIANCE);
    couture::hold(&mut cons, &[]);
    assert_eq!(cons.rest, plain.rest);
    assert_eq!(cons.compliance, plain.compliance);
    assert_eq!(
        xpbd::position_hash(&settle(&pipe, &cons)),
        xpbd::position_hash(&settle(&pipe, &plain)),
        "an empty set of elastics is not a set that rounds to nothing"
    );
}
