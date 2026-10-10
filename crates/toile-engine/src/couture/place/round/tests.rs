use super::*;

/// One half of a tube skirt: 0.20 m of waistline, flaring to `width` at the
/// hip and straight down from there to the hem.
fn half(width: f64) -> ShapePipeline {
    let contour = [
        [0.0, -0.50],
        [width, -0.50],
        [width, -0.20],
        [0.20, 0.0],
        [0.0, 0.0],
    ];
    ShapePipeline::build(&contour, 64, 1.0e-3).expect("the contour is finite")
}

/// A gore: `foot` of cloth at the bottom of the pattern, `head` at the top.
fn gore(foot: f64, head: f64, lo: f64, hi: f64) -> ShapePipeline {
    let contour = [[0.0, lo], [foot, lo], [head, hi], [0.0, hi]];
    ShapePipeline::build(&contour, 64, 1.0e-3).expect("the contour is finite")
}

/// A strap of one width, reaching from `lo` to `hi` and no further.
fn strap(width: f64, lo: f64, hi: f64) -> ShapePipeline {
    let contour = [[0.0, lo], [width, lo], [width, hi], [0.0, hi]];
    ShapePipeline::build(&contour, 64, 1.0e-3).expect("the contour is finite")
}

/// A parallelogram `width` across at every ordinate, its cloth standing `slant`
/// further along the pattern at `hi` than at `lo`.
///
/// The shape a slanted seam cuts, and the one a piece's width cannot describe:
/// it carries the same cloth at every height and none of it in the same place.
fn skewed(width: f64, slant: f64, lo: f64, hi: f64) -> ShapePipeline {
    let contour = [[0.0, lo], [width, lo], [width + slant, hi], [slant, hi]];
    ShapePipeline::build(&contour, 64, 1.0e-3).expect("the contour is finite")
}

/// The hoop at a given height is exactly as long as the cloth there, which
/// is the whole promise: the waistline and the hem both close, though one
/// carries half the cloth of the other.
#[test]
fn every_hoop_is_as_long_as_the_cloth_that_height_carries() {
    let (front, back) = (half(0.30), half(0.30));
    let pipes = [&front, &back];
    let round = Round::over(&[(0, 1.0), (1, -1.0)], &pipes, true, 0.005, None)
        .expect("both halves have a width");
    for k in 0..=10 {
        let y = -0.50 * f64::from(k) / 10.0;
        let hoop = TAU * round.radius(y);
        assert!(
            (hoop - round.girth(y)).abs() < 1.0e-12,
            "at {y} the hoop is {hoop} round and the cloth is {}",
            round.girth(y)
        );
    }
    assert!(
        round.girth(-0.50) > 1.4 * round.girth(0.0),
        "the hem carries far more cloth than the waistline: {} against {}",
        round.girth(-0.50),
        round.girth(0.0)
    );
}

/// The two ends of a closed strip meet: walking the whole cloth at any
/// ordinate comes back to where it started.
#[test]
fn a_closed_strip_comes_back_to_where_it_started() {
    let (front, back) = (half(0.30), half(0.30));
    let pipes = [&front, &back];
    let round = Round::over(&[(0, 1.0), (1, -1.0)], &pipes, true, 0.005, None)
        .expect("both halves have a width");
    // Read away from the very top and bottom, where the mesh's boundary
    // cuts the piece's corners and its edge stops being where the drawing
    // put it: the two sides of the centre seam are the pieces' own left
    // edges, and a cut corner is not one.
    for y in [-0.05, -0.10, -0.30, -0.45] {
        let (lo, hi) = (
            round.at(0, [0.0, y]).expect("the front is placed").0,
            round.at(1, [0.0, y]).expect("the back is placed").0,
        );
        // The front's abscissa zero and the back's are the two sides of the
        // centre seam, so they land a whole turn apart or on each other.
        let gap = (hi - lo).rem_euclid(TAU).min((lo - hi).rem_euclid(TAU));
        assert!(gap < 1.0e-9, "at {y} they are {gap} radians apart");
    }
}

/// Room asked for at one ordinate does not open the whole garment, and it
/// reaches its neighbours by no more than a band at a time.
#[test]
fn room_opens_the_bands_that_asked_and_leans_no_further() {
    let front = half(0.30);
    let pipes = [&front];
    let mut round =
        Round::over(&[(0, 1.0)], &pipes, false, 0.005, None).expect("the half has a width");
    let flat = round.radius(-0.25);
    assert!(round.open(&[-0.25], 0.005, 1.0), "the band opened");
    assert!(
        (round.radius(-0.25) - flat - 0.005).abs() < 1.0e-9,
        "the band it asked for opened by one step: {}",
        round.radius(-0.25) - flat
    );
    // And it leans no further than a band of radius per band of height, so
    // two bands away it has already run out.
    let two = round.radius(-0.24) - (round.girth(-0.24) / TAU);
    assert!(
        two < 1.0e-12,
        "two bands above the one that asked, nothing was opened: {two}"
    );
    let far = round.radius(-0.50) - (round.girth(-0.50) / TAU);
    assert!(
        far < 1.0e-12,
        "the far end of the piece was left alone: {far}"
    );
}

/// A ceiling stops the walk: a band already that far out is not opened
/// again, so a garment the field never reports clear cannot walk for ever.
#[test]
fn a_band_already_at_the_ceiling_is_not_opened_again() {
    let front = half(0.30);
    let pipes = [&front];
    let mut round =
        Round::over(&[(0, 1.0)], &pipes, false, 0.005, None).expect("the half has a width");
    let mut rounds = 0;
    while round.open(&[-0.25], 0.005, 0.02) {
        rounds += 1;
        assert!(rounds < 100, "the walk never stopped");
    }
    assert_eq!(rounds, 4, "it opened until the band reached the ceiling");
}

/// A piece nothing walks is not on the surface, and says so.
#[test]
fn a_piece_outside_the_walk_has_no_turn() {
    let front = half(0.30);
    let pipes = [&front];
    let round = Round::over(&[(0, 1.0)], &pipes, false, 0.005, None).expect("the half has a width");
    assert_eq!(round.at(1, [0.0, 0.0]), None);
    assert!(
        Round::over(&[], &pipes, false, 0.005, None).is_none(),
        "and a walk with nothing in it is no surface at all"
    );
}

/// Where the strip is whole the hoop and the cloth are one reading to the last
/// bit, which is the whole of what a hoop per ordinate buys.
#[test]
fn inside_the_whole_range_the_hoop_is_exactly_the_cloth() {
    let (one, two) = (gore(0.60, 0.10, 0.0, 1.00), strap(0.05, 0.20, 1.00));
    let pipes = [&one, &two];
    let round =
        Round::over(&[(0, 1.0), (1, -1.0)], &pipes, true, 0.005, None).expect("both have widths");
    let (lo, hi) = (
        round.whole.at(f64::NEG_INFINITY),
        round.whole.at(f64::INFINITY),
    );
    for k in 0..=20 {
        let y = lo + (hi - lo) * f64::from(k) / 20.0;
        assert!(
            (round.hoop(y) - round.girth(y)).abs() < 1.0e-12,
            "at {y} the strip is whole, and the hoop is {} where the cloth is {}",
            round.hoop(y),
            round.girth(y)
        );
    }
}

/// What the surface does above and below the range the strip is whole in.
mod flap;

/// What a declared heading does to the surface, and what it refuses to do.
mod pinned;
