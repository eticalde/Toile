#![allow(
    clippy::float_cmp,
    reason = "a sense is one of exactly two literals, so exact is the comparison"
)]

use super::*;

/// A rectangular panel `wide` metres across and `tall` metres down, drawn from
/// `from`: the plainest piece whose two side edges are the whole of two seams.
///
/// Rectangular on purpose. A seam drawn down a curve sits inside its piece's
/// widest point by an amount that varies along it, so no walk can put both of
/// its ends at one place on the strip; with straight edges the arithmetic below
/// has an exact answer and a test can hold the rule to it.
fn panel(from: f64, wide: f64, tall: f64) -> ShapePipeline {
    let rectangle = [
        [from, 0.0],
        [from + wide, 0.0],
        [from + wide, -tall],
        [from, -tall],
    ];
    ShapePipeline::build(&rectangle, 16, 0.02).expect("the rectangle is finite")
}

/// One seam, as the walk reads it: which two pieces it joins and where across
/// each of them it runs.
fn seam(sides: [usize; 2], at: [f64; 2]) -> Sewn {
    Sewn {
        sides,
        at,
        ends: [[0.0, 0.0], [0.0, 0.0]],
        a: Vec::new(),
        b: Vec::new(),
        dart: false,
    }
}

/// The seams as the walk is handed them: one ring's worth, by reference.
fn ring(sewn: &[Sewn]) -> Vec<&Sewn> {
    sewn.iter().collect()
}

/// How far along the strip, in metres of cloth, a pattern abscissa of one piece
/// lands.
///
/// The pieces abut in walk order, each taking as much of the strip as it is
/// wide, and the sense says which of a piece's two edges meets the piece
/// before it. This is the whole of what the sense decides, with no ring in it:
/// it is the same reading whether the cloth is later rolled onto a cylinder, a
/// cone or anything else, because rolling preserves length.
fn along(steps: &[Step], k: usize, x: f64) -> f64 {
    let before: f64 = steps[..k].iter().map(|s| s.hi - s.lo).sum();
    let step = &steps[k];
    before
        + if step.sense > 0.0 {
            x - step.lo
        } else {
            step.hi - x
        }
}

/// Holds a walk to what decides whether the garment can be sewn: the two sides
/// of every seam land at one place along the strip.
///
/// The reading [`crosses_forwards`] cannot give. That one compares a piece's
/// own two abscissae under the sense that piece was given, so it holds for any
/// order the pieces are laid in — orders leaving every seam a panel wide open
/// too. Zero here needs each piece both crossed the right way round and laid
/// beside the piece it is sewn to, and is zero rather than an inset only
/// because these fixtures draw their seams down the pieces' own edges.
fn shut(steps: &[Step], sewn: &[Sewn], closed: bool) {
    let girth: f64 = steps.iter().map(|s| s.hi - s.lo).sum();
    // A closed strip brings its two ends together, so the far way round counts.
    let round = if closed { girth } else { f64::INFINITY };
    for (j, one) in sewn.iter().enumerate() {
        let sides: Vec<f64> = steps
            .iter()
            .enumerate()
            .filter_map(|(k, step)| {
                let side = one.sides.iter().position(|&p| p == step.piece)?;
                Some(along(steps, k, one.at[side]))
            })
            .collect();
        assert_eq!(sides.len(), 2, "seam {j} joins two pieces the walk placed");
        let apart = (sides[1] - sides[0]).abs();
        let apart = apart.min(round - apart);
        assert!(
            apart < 1.0e-9,
            "seam {j} is handed to the solver {apart} m of cloth open: its two \
             sides land {} m and {} m along a strip {girth} m round",
            sides[0],
            sides[1]
        );
    }
}

/// Holds a walk to the two things it owes, whatever the pattern it was handed.
///
/// A seam lies in the cloth of both pieces it joins, and the walk crosses each
/// piece from the seam it arrived by to the seam it leaves by. So on the strip
/// every piece's entry has to come before its exit: a piece laid down the other
/// way round carries its exit seam back towards the piece it came from, which
/// is a garment folded onto itself rather than one that closes.
///
/// `seams` gives, per step, the abscissa the entry and the exit run at on that
/// piece.
fn crosses_forwards(steps: &[Step], seams: &[(f64, f64)]) {
    for (k, &(entry, exit)) in seams.iter().enumerate() {
        let step = &steps[k];
        for at in [entry, exit] {
            assert!(
                at >= step.lo && at <= step.hi,
                "a seam of piece {} runs at {at}, outside the {}..{} of cloth \
                 the piece carries",
                step.piece,
                step.lo,
                step.hi
            );
        }
        let (came, went) = (along(steps, k, entry), along(steps, k, exit));
        assert!(
            came < went,
            "piece {} is laid down back to front: it is entered {} m along the \
             strip and left {} m along it",
            step.piece,
            came,
            went
        );
    }
}

/// The walk over two pieces joined by two seams, with the second piece's edges
/// named by where they sit across it.
///
/// `side` and `inseam` are the abscissae the two seams run at on the second
/// piece; swapping those two is the whole of what drawing a piece mirrored
/// does to it.
fn tube(side: f64, inseam: f64) -> (Vec<Step>, Vec<(f64, f64)>) {
    let (front, back) = (panel(0.0, 0.60, 0.80), panel(1.00, 0.60, 0.80));
    let pipes = [&front, &back];
    // The front carries its inseam at low abscissa and its side seam at high,
    // which is how every one of these patterns is drawn.
    let sewn = [seam([0, 1], [0.60, side]), seam([0, 1], [0.00, inseam])];
    let joined = adjacency(&ring(&sewn), 2).expect("two pieces, two seams");
    let (steps, closed) = walk(&ring(&sewn), &pipes, &joined).expect("the strip closes");
    assert!(closed, "two pieces joined by two seams make a closed tube");
    shut(&steps, &sewn, closed);
    // The strip is opened at the front's side seam and closed at its inseam,
    // so the front is entered by the inseam and the back by the side seam.
    let seams = vec![(0.00, 0.60), (side, inseam)];
    (steps, seams)
}

/// Three pieces sewn in a ring, walked and held to `senses`: the first count at
/// which a walk can lay a piece beside one it is not sewn to.
///
/// `at` gives, per seam, the abscissa it runs at on each of its two pieces.
fn loop_of_three(at: [[f64; 2]; 3], senses: [f64; 3]) {
    let cut: Vec<ShapePipeline> = (0..3).map(|k| panel(f64::from(k), 0.60, 0.80)).collect();
    let pipes: Vec<&ShapePipeline> = cut.iter().collect();
    let sewn = vec![
        seam([0, 1], at[0]),
        seam([1, 2], at[1]),
        seam([2, 0], at[2]),
    ];
    let joined = adjacency(&ring(&sewn), 3).expect("three pieces, three seams");
    let (steps, closed) = walk(&ring(&sewn), &pipes, &joined).expect("the loop closes");
    assert!(closed, "three pieces joined in a ring make a closed strip");
    assert_eq!(steps.iter().map(|s| s.sense).collect::<Vec<_>>(), senses);
    shut(&steps, &sewn, closed);
}

/// A piece drawn mirrored runs the same way round the body as the one before
/// it, and that is the answer that sews the two shut.
///
/// This is a cutter's own layout: the back laid out flipped, so its inseam
/// sits at high abscissa where the front's sits at low. Walking the strip then
/// ascends the abscissa on both pieces and both take the same sense — not a
/// failure to alternate, but the only way round that leaves each piece's cloth
/// between the seam it was entered by and the seam it is left by. A rule that
/// made the senses alternate whatever the pattern said would fold this garment
/// onto itself.
#[test]
fn a_mirrored_piece_runs_the_same_way_round_as_the_one_before_it() {
    let (steps, seams) = tube(1.00, 1.60);
    assert_eq!(
        steps.iter().map(|s| s.sense).collect::<Vec<_>>(),
        [1.0, 1.0]
    );
    crosses_forwards(&steps, &seams);
}

/// A piece drawn the same way up as the one before it runs against it, and
/// that is the answer that sews those two shut.
///
/// The shipped block and both reference skirts are drawn like this, so a rule
/// that gave every piece the same sense would fold all three.
#[test]
fn a_piece_drawn_the_same_way_up_runs_against_the_one_before_it() {
    let (steps, seams) = tube(1.60, 1.00);
    assert_eq!(
        steps.iter().map(|s| s.sense).collect::<Vec<_>>(),
        [1.0, -1.0]
    );
    crosses_forwards(&steps, &seams);
}

/// Whichever way the second piece is drawn, the first runs with the turn.
///
/// The tie-break `opening` takes, and the whole of what keeps a garment from
/// coming out mirrored: a closed strip is left by whichever of its first
/// piece's seams sits further along that piece, so that piece's abscissa rises
/// with the turn.
#[test]
fn the_first_piece_of_a_closed_strip_runs_with_the_turn() {
    for (side, inseam) in [(1.00, 1.60), (1.60, 1.00)] {
        let (steps, _) = tube(side, inseam);
        assert_eq!(
            steps[0].sense, 1.0,
            "the strip was opened by the wrong seam of its first piece"
        );
    }
}

/// A piece at the end of an open strip is turned by its one seam, and the cloth
/// past that seam reaches to the piece's own far edge.
///
/// The case with no entry seam to read, and the mirroring still has to come out
/// right: the front's one seam is at its high abscissa and the back's at its
/// low, so both pieces run with the turn — the same answer as the closed tube,
/// reached without a second seam to compare against.
#[test]
fn an_end_piece_is_turned_by_the_cloth_beyond_its_one_seam() {
    let (front, back) = (panel(0.0, 0.60, 0.80), panel(1.00, 0.60, 0.80));
    let pipes = [&front, &back];
    let sewn = [seam([0, 1], [0.60, 1.00])];
    let joined = adjacency(&ring(&sewn), 2).expect("two pieces, one seam");
    let (steps, closed) = walk(&ring(&sewn), &pipes, &joined).expect("the strip walks");
    assert!(!closed, "one seam leaves the strip open at both ends");
    assert_eq!(
        steps.iter().map(|s| s.sense).collect::<Vec<_>>(),
        [1.0, 1.0]
    );
    crosses_forwards(&steps, &[(0.00, 0.60), (1.00, 1.60)]);
    shut(&steps, &sewn, closed);
}

/// Three pieces are laid beside the pieces they are sewn to, whichever way
/// round any one of them is drawn.
///
/// Two pieces cannot say this: with two, every order is the right order up to
/// the mirror the first piece's tie-break already fixes. With three, a walk
/// that chained the seams in the wrong order would still cross each piece from
/// its entry to its exit, and leave all three seams a panel wide open.
#[test]
fn three_pieces_land_beside_the_pieces_they_are_sewn_to() {
    // Every piece drawn the same way up, which gives all three one sense.
    loop_of_three([[0.60, 1.00], [1.60, 2.00], [2.60, 0.00]], [1.0, 1.0, 1.0]);
    // And the middle one mirrored, which turns that one alone.
    loop_of_three([[0.60, 1.60], [1.00, 2.00], [2.60, 0.00]], [1.0, -1.0, 1.0]);
}

/// A piece whose two seams run at one abscissa is refused, not guessed at.
///
/// They name no end of the piece for the walk to come in by, and the two
/// answers are a hair apart in the arithmetic and half a garment apart on the
/// body. Refusing is what the walk already answers a piece sewn to itself.
#[test]
fn a_piece_whose_two_seams_run_at_one_abscissa_is_refused() {
    let (front, back) = (panel(0.0, 0.60, 0.80), panel(1.00, 0.60, 0.80));
    let pipes = [&front, &back];
    let middle = 1.30;
    let sewn = [seam([0, 1], [0.60, middle]), seam([0, 1], [0.00, middle])];
    let joined = adjacency(&ring(&sewn), 2).expect("two pieces, two seams");
    assert!(
        walk(&ring(&sewn), &pipes, &joined).is_none(),
        "the back's two seams run at one place on it, so it has no sense"
    );
    // A hair either side of that is answered, and answered differently, which
    // is why the tie cannot be broken by picking one.
    for step in [1.0e-9, -1.0e-9] {
        let sewn = [
            seam([0, 1], [0.60, middle]),
            seam([0, 1], [0.00, middle + step]),
        ];
        let (steps, _) = walk(&ring(&sewn), &pipes, &joined).expect("the strip closes");
        assert_eq!(steps[1].sense, if step > 0.0 { 1.0 } else { -1.0 });
    }
}

/// The cloth a piece carries across is measured between its own edges, and a
/// seam running down it is inside that cloth rather than the end of it.
#[test]
fn a_piece_carries_the_cloth_between_its_own_edges() {
    let panel = panel(0.25, 0.40, 0.30);
    let (lo, hi) = across(&panel);
    assert!((lo - 0.25).abs() < 1.0e-9, "{lo}");
    assert!((hi - 0.65).abs() < 1.0e-9, "{hi}");
}
