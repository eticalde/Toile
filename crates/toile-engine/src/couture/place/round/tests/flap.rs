use super::*;

/// Where the cloth runs furthest past its hoop over a whole surface, and the
/// ordinate it happens at.
fn worst_shortfall(round: &Round) -> (f64, f64) {
    let (from, to) = round.span();
    let mut worst = (f64::MIN, from);
    for k in 0..=2000 {
        let y = from + (to - from) * f64::from(k) / 2000.0;
        let short = round.girth(y) - round.hoop(y);
        if short > worst.0 {
            worst = (short, y);
        }
    }
    worst
}

/// Prints that reading and hands back how far short the worst hoop came.
///
/// Printed before anything is judged, and judged by the caller once it has read
/// every arrangement: an arrangement that fails is one number, and the reading
/// beside it is what says whether the others were near it or nowhere close.
fn shortfall_of(name: &str, round: &Round) -> f64 {
    let (short, y) = worst_shortfall(round);
    println!(
        "{name}: the cloth comes closest to outrunning its hoop at {y:.5}, \
         {:.6} m of cloth on {:.6} m of hoop — {:.1} % of it, {:.9} m to spare",
        round.girth(y),
        round.hoop(y),
        100.0 * round.hoop(y) / round.girth(y),
        -short
    );
    short
}

/// No hoop of the surface is shorter than the cloth that height carries.
///
/// The one answer a placement does not have. Rolling preserves length along a
/// hoop, so cloth that will not go round one has nowhere to be but inside the
/// next piece — which is what `product::arcs` refuses by reading the order the
/// pieces come in and what `fit.rs` holds both bodies' skirts to.
///
/// Two arrangements, because the strip's whole range runs out in two different
/// ways. Pieces that overlap most of the garment leave a stretch below the
/// range where one of them carries cloth and the other does not. Pieces whose
/// spans leave a gap have no common height at all, so the range collapses to a
/// single ordinate and every hoop of the garment is read there.
#[test]
fn no_hoop_is_shorter_than_the_cloth_that_height_carries() {
    let (one, two) = (gore(0.60, 0.10, 0.0, 1.00), strap(0.05, 0.20, 1.00));
    let pipes = [&one, &two];
    let round = Round::over(&[(0, 1.0), (1, -1.0)], &pipes, true, 0.005).expect("both have widths");
    let overlapping = shortfall_of(
        "a gore beside a strap starting a fifth of the way up",
        &round,
    );

    let (low, tall, high) = (
        gore(0.50, 0.20, 0.0, 0.40),
        gore(0.90, 0.05, 0.0, 1.00),
        strap(0.10, 0.60, 1.00),
    );
    let pipes = [&low, &tall, &high];
    let round = Round::over(&[(0, 1.0), (1, -1.0), (2, 1.0)], &pipes, true, 0.005)
        .expect("all three have widths");
    assert!(
        round.whole.at(f64::NEG_INFINITY) >= round.whole.at(f64::INFINITY),
        "the three spans leave a gap, so the strip is whole nowhere and every \
         hoop of it is one reading"
    );
    let gapped = shortfall_of("three pieces whose spans leave a gap", &round);

    for (name, short) in [("overlapping", overlapping), ("gapped", gapped)] {
        assert!(
            short <= 1.0e-12,
            "{name}: the cloth outruns its hoop by {short} m, so the pieces have \
             nowhere to be on it but inside each other"
        );
    }
}

/// And it is still the last whole hoop that the surface carries on, so there is
/// no cliff for the cloth to fold at.
///
/// The floor only ever adds. Below the strip's whole range every panel is read
/// at the same held ordinate, so the hoop there is the whole strip's own last
/// one and the floor can raise it but not drop it. Letting the hoop follow the
/// cloth that is left instead drops it by a whole piece's width the moment that
/// piece ends, and no clearance table leaning a band of radius per band of
/// height can bridge that.
#[test]
fn below_the_whole_range_the_hoop_never_pinches_inside_the_last_whole_one() {
    let (one, two) = (gore(0.60, 0.10, 0.0, 1.00), strap(0.05, 0.20, 1.00));
    let pipes = [&one, &two];
    let round = Round::over(&[(0, 1.0), (1, -1.0)], &pipes, true, 0.005).expect("both have widths");
    let lo = round.whole.at(f64::NEG_INFINITY);
    let (from, _) = round.span();
    let last = round.hoop(lo);
    for k in 0..=200 {
        let y = from + (lo - from) * f64::from(k) / 200.0;
        assert!(
            round.hoop(y) >= last - 1.0e-12,
            "at {y}, below the whole range, the hoop pinched to {} inside the last \
             whole one at {last}",
            round.hoop(y)
        );
        // And the cloth that is left is what raised it, so the two readings
        // meet where the shorter piece runs out and part where it does
        // not.
        assert!(
            (round.hoop(y) - last).abs() < 1.0e-12 || round.hoop(y) > round.girth(y),
            "at {y} the hoop left the last whole one for no cloth: {} against \
             {last}, carrying {}",
            round.hoop(y),
            round.girth(y)
        );
    }
    println!(
        "the last whole hoop is {last:.6} m and the foot of the gore lands on \
         {:.6} m, carrying {:.6} m",
        round.hoop(from),
        round.girth(from)
    );
}

/// Every vertex is laid on the arc its own panel was given, so no piece is laid
/// inside its neighbour.
///
/// The arc a vertex is measured into has to be an arc that vertex is on, and a
/// hoop long enough does not give that on its own. A slanted panel's cloth
/// stands further along the pattern the higher it is read, so above the strip's
/// whole range the held reading — taken at one ordinate for the whole flap —
/// need not reach the edges the cloth really has there. Measured from an edge
/// the cloth has left, a vertex walks off its own arc and onto its neighbour's,
/// which is one piece inside the other at a height where both are there.
///
/// Measured on the arrangement below: with the arc merely picked between the
/// two readings, a vertex of the parallelogram lands 0.300 m along a 0.300 m
/// arc, and the seam it shares with the strap beside it is laid 180 mm open.
#[test]
fn no_vertex_is_laid_outside_the_arc_its_own_panel_was_given() {
    let (slanted, mid, low) = (
        skewed(0.30, 0.60, 0.0, 1.00),
        strap(0.30, 0.0, 0.80),
        strap(0.30, 0.0, 0.50),
    );
    let pipes = [&slanted, &mid, &low];
    let round = Round::over(&[(0, 1.0), (1, -1.0), (2, 1.0)], &pipes, true, 0.005)
        .expect("all three have widths");
    let (mut before, mut past) = (0.0f64, 0.0f64);
    for panel in &round.panels {
        for &p in &pipes[panel.piece].pos2d {
            let (lo, hi) = round.reach(panel, p[1]);
            let into = if panel.sense > 0.0 {
                p[0] - lo
            } else {
                hi - p[0]
            };
            before = before.max(-into);
            past = past.max(into - (hi - lo));
        }
    }
    println!(
        "over a strip carrying a panel slanted 0.60 m across its own height, no \
         vertex runs {before:.9} m before its arc or {past:.9} m past it"
    );
    assert!(
        before <= 0.0 && past <= 0.0,
        "a vertex is laid {before} m before its own arc and {past} m past it, so \
         that much of this panel is on a neighbour's stretch of the surface"
    );
}

/// Above the whole range the piece that is left is carried up the surface, not
/// sheared round it.
///
/// A vertical edge of the tall piece is one side of it all the way up, so on a
/// surface carried straight up it comes out a meridian and its turn does not
/// change. Read the panels at their own ordinate instead and a narrowing piece
/// moves the meridian the strip opens at with every row of vertices, which
/// turns the free flap round the body as it goes. The shipped block reads this
/// on the 25 mm its back rises above its front; here it is a whole tenth of a
/// metre, and both come from the same line of the surface.
#[test]
fn above_the_whole_range_the_piece_left_is_carried_up_and_not_turned() {
    let (tall, short) = (gore(0.30, 0.02, 0.0, 1.10), strap(0.30, 0.0, 1.00));
    let pipes = [&tall, &short];
    let round = Round::over(&[(0, 1.0), (1, -1.0)], &pipes, true, 0.005).expect("both have widths");
    let hi = round.whole.at(f64::INFINITY);
    let (_, top) = round.span();
    let mut swing = 0.0f64;
    let at_the_line = round.at(0, [0.0, hi]).expect("the tall piece is placed").0;
    for k in 0..=100 {
        let y = hi + (top - hi) * f64::from(k) / 100.0;
        let (turn, _) = round.at(0, [0.0, y]).expect("the tall piece is placed");
        swing = swing.max((turn - at_the_line).abs());
    }
    println!(
        "over the {:.1} mm above the whole range the tall piece's own edge swings \
         {swing:.3e} rad, on a hoop falling from {:.6} m to {:.6} m of cloth",
        (top - hi) * 1000.0,
        round.girth(hi),
        round.girth(top)
    );
    assert!(
        swing < 1.0e-9,
        "the piece left above the strip is sheared round the surface rather than \
         carried up it: its own edge swings {swing} rad over {:.1} mm of pattern",
        (top - hi) * 1000.0
    );
}
