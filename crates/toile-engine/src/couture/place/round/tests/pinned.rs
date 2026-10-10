use std::f64::consts::PI;

use super::*;

/// Two straps of 0.25 m, a strip 0.50 m round, read at an ordinate both carry.
fn tube() -> [ShapePipeline; 2] {
    [strap(0.25, -0.40, 0.0), strap(0.25, -0.40, 0.0)]
}

/// The pin that puts abscissa `at` of piece `piece` at `turn` radians, running
/// the piece's rising abscissa leftward.
fn pin(piece: usize, at: f64, turn: f64) -> Pin {
    Pin {
        piece,
        at,
        turn,
        leftward: 1.0,
    }
}

/// The reading a declaration asks for is the reading it gets: the pinned
/// abscissa comes back at the turn it was pinned at, exactly.
///
/// Exactly and not nearly, which is what makes a heading a declaration rather
/// than a hint: the pin opens the turn, so the arithmetic at that abscissa is
/// `turn + 0`.
#[test]
fn the_pinned_abscissa_comes_back_at_the_turn_it_was_pinned_at() {
    let [a, b] = tube();
    let pipes = [&a, &b];
    for turn in [0.0, PI / 2.0, PI, -PI / 2.0, 0.37] {
        let round = Round::over(
            &[(0, 1.0), (1, 1.0)],
            &pipes,
            true,
            0.005,
            Some(pin(0, 0.1, turn)),
        )
        .expect("both straps have a width");
        let (got, _) = round.at(0, [0.1, -0.20]).expect("the strip carries it");
        assert!((got - turn).abs() < 1.0e-12, "{turn} came back {got}");
    }
}

/// And the same turn is the same place at every ordinate, which is what makes a
/// declared heading a rigid turn of the surface rather than a shift of cloth.
///
/// Read down a piece whose width changes with height, so a reading that had
/// been taken once at the crest, or added as metres of cloth, would walk: the
/// gore carries 0.60 m at its foot and 0.10 m at its head.
#[test]
fn a_pinned_line_is_a_meridian_down_the_whole_surface() {
    let (one, two) = (gore(0.60, 0.10, -1.0, 0.0), gore(0.10, 0.60, -1.0, 0.0));
    let pipes = [&one, &two];
    let round = Round::over(
        &[(0, 1.0), (1, 1.0)],
        &pipes,
        true,
        0.005,
        Some(pin(0, 0.0, PI / 3.0)),
    )
    .expect("both gores have widths");
    for k in 0..=20 {
        let y = -1.0 + f64::from(k) / 20.0;
        let (turn, _) = round.at(0, [0.0, y]).expect("the strip carries it");
        assert!(
            (turn - PI / 3.0).abs() < 1.0e-12,
            "at {y} the pinned line sits at {turn} rather than {}",
            PI / 3.0
        );
    }
}

/// A pin whose piece the walk does not carry is dropped, and the surface is the
/// one every release had before a heading could be declared.
///
/// Dropped and not carried, because `at` would match no panel: the turn would
/// open at a cloth of zero and the garment come out turned by whatever the
/// storage order gave it, with nothing said. Compared bit for bit against the
/// same surface built with no pin at all, which is the claim.
#[test]
fn a_pin_on_a_piece_the_walk_does_not_carry_is_dropped() {
    let [a, b] = tube();
    let pipes = [&a, &b];
    let over =
        |pin| Round::over(&[(0, 1.0)], &pipes, false, 0.005, pin).expect("the strap has a width");
    let (stray, bare) = (over(Some(pin(1, 0.1, PI))), over(None));
    for k in 0..=10 {
        let p = [0.25 * f64::from(k) / 10.0, -0.20];
        assert_eq!(
            stray.at(0, p).map(|(turn, _)| turn.to_bits()),
            bare.at(0, p).map(|(turn, _)| turn.to_bits()),
            "at {p:?}"
        );
    }
}

/// The sense sends the strip round the other way without moving the pin, and
/// every piece's cloth runs against the turn when it does.
///
/// Both halves, because either alone is half the reading: the pinned line stays
/// where it was put, and the cloth either side of it swaps.
#[test]
fn the_sense_turns_the_strip_round_the_other_way_from_the_same_pin() {
    let [a, b] = tube();
    let pipes = [&a, &b];
    let over = |leftward| {
        Round::over(
            &[(0, 1.0), (1, 1.0)],
            &pipes,
            true,
            0.005,
            Some(Pin {
                leftward,
                ..pin(0, 0.0, 0.0)
            }),
        )
        .expect("both straps have a width")
    };
    let (left, right) = (over(1.0), over(-1.0));
    for k in 1..=10 {
        let p = [0.25 * f64::from(k) / 10.0, -0.20];
        let (one, _) = left.at(0, p).expect("the strip carries it");
        let (other, _) = right.at(0, p).expect("and so does the mirror");
        assert!(one > 0.0, "leftward walks the turn up: {one}");
        assert!((one + other).abs() < 1.0e-12, "{one} against {other}");
    }
    assert!(left.strip().all(|(_, sense)| sense > 0.0));
    assert!(
        right.strip().all(|(_, sense)| sense < 0.0),
        "every piece's cloth runs against the turn"
    );
}

/// The reading goes through the placement, so it answers in the lap a heading
/// is declared in and for no piece the strip does not carry.
#[test]
fn facing_reads_the_placement_back_in_the_lap_a_heading_is_written_in() {
    let [a, b] = tube();
    let pipes = [&a, &b];
    let round = Round::over(
        &[(0, 1.0), (1, 1.0)],
        &pipes,
        true,
        0.005,
        Some(pin(0, 0.0, PI)),
    )
    .expect("both straps have a width");
    // The pin is at the centre back and the second strap opens half a lap on
    // from it, so that edge is the front — and the reading has to come back
    // folded into the lap rather than at 360.
    assert!((round.facing(0, 0.0, -0.20).expect("pinned") - 180.0).abs() < 1.0e-9);
    let across = round.facing(1, 0.0, -0.20).expect("the second strap");
    assert!(across.abs() < 1.0e-9, "the far side reads {across}");
    assert_eq!(round.facing(2, 0.0, -0.20), None, "no third piece");
}
