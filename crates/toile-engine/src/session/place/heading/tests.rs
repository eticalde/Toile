use super::*;

/// One seam, as the inheritance reads it: which two pieces it joins and the
/// abscissae its sewn stretch begins and ends at on each of them.
fn seam(sides: [usize; 2], ends: [[f64; 2]; 2]) -> Sewn {
    Sewn {
        sides,
        at: [0.0, 0.0],
        ends,
        a: Vec::new(),
        b: Vec::new(),
        dart: false,
    }
}

/// A heading declared on one piece, pinned at one abscissa of it.
fn pin(piece: usize, at: f64) -> Pin {
    Pin {
        piece,
        at,
        turn: 0.0,
        leftward: 1.0,
    }
}

/// Where each piece came out pinned, and which way its cloth was said to run.
fn read(found: &[Option<Turned>]) -> Vec<Option<(f64, f64)>> {
    found
        .iter()
        .map(|one| one.map(|one| (one.pin.at, one.pin.leftward)))
        .collect()
}

/// A heading written on a piece is that piece's own, and it reaches the piece
/// across the seam at the stretch's own scale.
///
/// The collar strip of the bench's shirt, in numbers: its lower edge is sewn
/// to the left front's chest line, the two stretches run the same way, and the
/// line declared at the front's abscissa 0 is the strip's own 3.10 m.
#[test]
fn a_heading_crosses_a_seam_onto_the_line_of_cloth_it_is_sewn_to() {
    let sewn = [seam([0, 1], [[0.0, 0.26], [3.10, 3.36]])];
    let found = spread(&sewn, 2, &[pin(0, 0.0)]);
    assert_eq!(read(&found), [Some((0.0, 1.0)), Some((3.10, 1.0))]);
    assert_eq!(found[0].expect("it is declared there").seams, 0);
    assert_eq!(found[1].expect("and carried one seam out").seams, 1);
}

/// Two stretches that run against each other carry the sense reversed, which
/// is what a sense is about: the piece's own rising abscissa.
#[test]
fn a_seam_whose_sides_run_against_each_other_reverses_the_sense() {
    let sewn = [seam([0, 1], [[0.0, 0.26], [3.36, 3.10]])];
    let found = spread(&sewn, 2, &[pin(0, 0.0)]);
    assert_eq!(read(&found)[1], Some((3.36, -1.0)));
}

/// A seam that runs at one abscissa carries nothing, and the piece on its far
/// side is left to the placement it always had.
///
/// The side seam of any garment: it says where two panels meet and nothing at
/// all about how far across either of them another line is.
#[test]
fn a_seam_that_runs_at_one_abscissa_carries_no_heading() {
    let sewn = [seam([0, 1], [[0.26, 0.26], [0.0, 0.0]])];
    let found = spread(&sewn, 2, &[pin(0, 0.0)]);
    assert_eq!(read(&found), [Some((0.0, 1.0)), None]);
}

/// A piece between two declarations takes the nearer one, and a piece the same
/// distance from both takes the one that hangs higher.
///
/// Highest first is the order the document's own pins arrive in, so the rank
/// is what settles a tie and never which seam the walk met first.
#[test]
fn the_nearer_declaration_wins_and_a_tie_goes_to_the_higher_one() {
    let sewn = [
        seam([0, 1], [[0.0, 0.1], [1.0, 1.1]]),
        seam([1, 2], [[1.0, 1.1], [2.0, 2.1]]),
        seam([2, 3], [[2.0, 2.1], [3.0, 3.1]]),
        seam([0, 3], [[0.0, 0.1], [3.0, 3.1]]),
    ];
    let found = spread(&sewn, 4, &[pin(0, 0.0), pin(2, 2.0)]);
    let at: Vec<Option<f64>> = read(&found)
        .iter()
        .map(|one| one.map(|one| one.0))
        .collect();
    assert_eq!(at, [Some(0.0), Some(1.0), Some(2.0), Some(3.0)]);
    // Pieces 1 and 3 each sit one seam from both declarations; both come out
    // carrying the first pin, which is the highest the document holds.
    for piece in [1, 3] {
        assert_eq!(found[piece].expect("it is reached").rank, 0, "{piece}");
        assert_eq!(found[piece].expect("one seam out").seams, 1, "{piece}");
    }
}

/// A declaration standing much further from a seam than the seam is wide is
/// not carried through it.
///
/// A trouser's outside leg seam in numbers: it crosses 1.3 cm of abscissa from
/// the waist corner to the hem corner, and the centre front it would carry
/// stands 22 cm away. The correspondence is only told at the stretch's two
/// ends, so read sixteen stretch-widths out it multiplies whatever the two
/// sides disagree about and lands the centre front wherever that ratio says.
#[test]
fn a_declaration_far_outside_a_narrow_seam_is_not_carried() {
    let sewn = [seam([0, 1], [[0.22, 0.2069], [0.92, 0.9069]])];
    assert_eq!(read(&spread(&sewn, 2, &[pin(0, 0.0)]))[1], None);
    // The same seam carrying a line of its own: inside the stretch it is a
    // reading and not a ratio, so this one does cross.
    let near = read(&spread(&sewn, 2, &[pin(0, 0.21)]));
    assert_eq!(near[1].map(|one| (one.0 * 1.0e4).round()), Some(9100.0));
}
