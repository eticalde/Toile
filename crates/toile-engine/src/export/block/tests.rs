use super::*;

/// A piece as a rectangle of cloth, from the box it fills.
fn cloth([x, y, far, down]: [f64; 4]) -> Vec<[f64; 2]> {
    vec![[x, y], [far, y], [far, down], [x, down]]
}

/// A sheet with nothing on it yet, and one piece in the middle of it.
fn room<'a>(
    cloth: &'a [[f64; 2]],
    taken: &'a [[f64; 4]],
    others: &'a [&'a [[f64; 2]]],
) -> Room<'a> {
    Room {
        sheet: [0.0, 0.0, 190.0, 220.0],
        cloth,
        taken,
        others,
    }
}

/// What every piece of the owner's jeans says, at the sizes the sheet sets
/// them.
fn said() -> Vec<(f64, &'static str)> {
    vec![
        (5.0, "G · «VISTA BRAGUETA»"),
        (3.0, "5.0 × 17.0 cm"),
        (3.0, "Cortar 2 · margen 1 cm por fuera del contorno"),
    ]
}

/// A line too wide for its piece is broken, and every word of it survives.
///
/// The piece is the narrowest of the owner's jeans at five centimetres, and
/// its cutting instruction is seven wide. Abbreviating it is what a person at
/// a cutting table reads wrong, and half a line printed still reads.
#[test]
fn a_line_wider_than_its_piece_is_broken_and_never_shortened() {
    let laid = laid(&said(), &room(&cloth([20.0, 10.0, 70.0, 180.0]), &[], &[]))
        .expect("the block fits somewhere on the sheet");
    let whole: Vec<String> = laid.iter().map(|line| line.body.clone()).collect();
    assert!(laid.len() > said().len(), "{whole:?} was not broken at all");
    assert_eq!(
        whole.join(" "),
        "G · «VISTA BRAGUETA» 5.0 × 17.0 cm Cortar 2 · margen 1 cm por fuera del contorno"
    );
    for line in &laid {
        assert!(
            line.at[0] >= 20.0,
            "«{}» opens left of its piece",
            line.body
        );
    }
}

/// And a line that fits is passed through whole, at the place the block opens.
#[test]
fn a_block_with_room_for_it_opens_at_its_own_corner_unbroken() {
    let laid = laid(&said(), &room(&cloth([20.0, 10.0, 160.0, 180.0]), &[], &[]))
        .expect("a wide piece has room");
    let whole: Vec<&str> = laid.iter().map(|line| line.body.as_str()).collect();
    assert_eq!(
        whole,
        [
            "G · «VISTA BRAGUETA»",
            "5.0 × 17.0 cm",
            "Cortar 2 · margen 1 cm por fuera del contorno",
        ]
    );
    assert!(
        (laid[0].at[0] - (20.0 + INSET)).abs() < 1e-9,
        "{:?}",
        laid[0].at
    );
    assert!((laid[0].at[1] - 15.0).abs() < 1e-9, "{:?}", laid[0].at);
}

/// The block gives way to a name already written where it wanted to open, and
/// drops to the first gap clear of it — not to the bottom of the piece, and
/// not on top of the name.
///
/// A node name is a coordinate a person matches between two taped sheets; the
/// block is prose the legend says too. So the block is the one that moves.
#[test]
fn a_block_drops_under_a_name_already_written_across_its_corner() {
    let name = [[22.0, 11.0, 48.0, 16.0]];
    let laid = laid(
        &said(),
        &room(&cloth([20.0, 10.0, 160.0, 180.0]), &name, &[]),
    )
    .expect("there is room under the name");
    assert!(laid[0].at[1] > 16.0, "the title still sits on the name");
    for line in &laid {
        let held = metric::box_of(line.size, line.at, &line.body);
        assert!(!meets(held, name[0]), "«{}» is on the name", line.body);
    }
    // And no further than it had to: the gap under the name, not the floor.
    assert!(laid[0].at[1] < 30.0, "{:?} dropped too far", laid[0].at);
}

/// A name beside the block and not across it costs the block nothing: a block
/// that dropped under every name on the sheet would end up nowhere near the
/// piece it names.
#[test]
fn a_name_the_block_does_not_reach_does_not_move_it() {
    let beside = [[120.0, 11.0, 150.0, 16.0]];
    let laid = laid(
        &said(),
        &room(&cloth([20.0, 10.0, 70.0, 180.0]), &beside, &[]),
    )
    .expect("the block fits");
    assert!((laid[0].at[1] - 15.0).abs() < 1e-9, "{:?}", laid[0].at);
}

/// A block that could only be written inside another piece's cut line is not
/// written at all: paper that says `Cortar 2` without saying whose is worse
/// than paper that says it one sheet less often.
#[test]
fn a_block_that_would_land_on_another_piece_is_left_off_the_sheet() {
    let neighbour: Vec<[f64; 2]> = vec![[28.0, 0.0], [190.0, 0.0], [190.0, 200.0], [28.0, 200.0]];
    let others: Vec<&[[f64; 2]]> = vec![&neighbour];
    assert!(
        laid(
            &said(),
            &room(&cloth([20.0, 10.0, 26.0, 180.0]), &[], &others)
        )
        .is_none()
    );
    // The same block, with that neighbour out of the way, is written.
    assert!(laid(&said(), &room(&cloth([20.0, 10.0, 26.0, 180.0]), &[], &[])).is_some());
}

/// A piece whose box this sheet does not reach says nothing on it, rather than
/// saying it at the edge of somebody else's paper.
#[test]
fn a_piece_the_sheet_does_not_carry_says_nothing_on_it() {
    assert!(
        laid(
            &said(),
            &room(&cloth([200.0, 10.0, 260.0, 180.0]), &[], &[])
        )
        .is_none()
    );
}

/// A block opens on its own cloth and not at the corner of the box around it.
///
/// The box around a tiled piece can cover the whole sheet while the cloth on
/// that sheet is a tongue down one side of it: a block laid at that box's own
/// corner is a block on paper the scissors take away. Here the cloth is a
/// column down the right of the box with a thread of it along the bottom, so
/// the corner the block would rather have is a hand's width from any cloth.
#[test]
fn a_block_runs_right_until_it_stands_on_its_own_cloth() {
    let tongue = [
        [20.0, 205.0],
        [190.0, 205.0],
        [190.0, 0.0],
        [150.0, 0.0],
        [150.0, 200.0],
        [20.0, 200.0],
    ];
    let laid = laid(&said(), &room(&tongue, &[], &[])).expect("the tongue has room for it");
    assert!(
        home(&laid, &tongue) > 0.5,
        "{:.3} of the ink landed on the cloth",
        home(&laid, &tongue)
    );
    assert!(
        laid[0].at[0] > 100.0,
        "the block stayed at {:?}",
        laid[0].at
    );
}

/// And a piece too small to stand its own words on says them beside itself
/// rather than not at all.
///
/// A strip fifteen centimetres long and eight millimetres tall cannot hold a
/// block two centimetres deep at any place on any sheet. The other answer is
/// a sheet carrying a nameless outline, and this one is the decision the
/// narrowest piece of the owner's jeans was already taken under.
#[test]
fn a_piece_too_small_for_its_own_words_still_says_them_beside_itself() {
    let strip = cloth([20.0, 10.0, 170.0, 18.0]);
    let laid = laid(&said(), &room(&strip, &[], &[])).expect("the strip still says what it is");
    assert_eq!(laid.len(), said().len(), "the block was broken or dropped");
    let held = home(&laid, &strip);
    assert!(
        held < HOME,
        "the strip is too shallow to hold all of it, and held {held:.3}"
    );
}

/// A name that touches the box around the block and no word in it leaves the
/// block where it is.
///
/// The block is three lines, and the middle one is a third the width of the
/// longest. A name in the gap that leaves is a name on nothing, and sending
/// the block down the sheet for it is paying a centimetre of paper for a
/// collision that was never there.
#[test]
fn a_name_in_the_gap_a_short_line_leaves_does_not_move_the_block() {
    let hole = [[45.0, 17.0, 60.0, 20.0]];
    let laid = laid(
        &said(),
        &room(&cloth([20.0, 10.0, 160.0, 180.0]), &hole, &[]),
    )
    .expect("the block fits");
    assert!((laid[0].at[1] - 15.0).abs() < 1e-9, "{:?}", laid[0].at);
    for line in &laid {
        let held = metric::box_of(line.size, line.at, &line.body);
        assert!(!meets(held, hole[0]), "«{}» is on the name", line.body);
    }
}

/// A block with no room left under the names is left off too, and that is the
/// count the product reports.
#[test]
fn a_block_with_no_gap_left_under_the_names_is_left_off() {
    let packed: Vec<[f64; 4]> = (0..44)
        .map(|rank| {
            let down = 10.0 + 5.0 * f64::from(rank);
            [20.0, down, 160.0, down + 5.0]
        })
        .collect();
    assert!(
        laid(
            &said(),
            &room(&cloth([20.0, 10.0, 160.0, 180.0]), &packed, &[])
        )
        .is_none()
    );
}
