use super::super::metric;
use super::super::units::{CAPTION, MARGIN};
use super::grid::{OVERLAP, cell};
use super::paper::points;
use super::read::{path_of, streams_of};
use super::tests::{ell, rectangle};
use super::text::literal;
use super::{A4, piece_to_pdf};

/// A piece too big for the paper is laid across more paper. Scaling it down
/// would print a sheet that looks right and cuts a garment two sizes off, and
/// refusing it would leave the owner where he was: unable to print a leg.
#[test]
fn a_piece_larger_than_the_paper_is_tiled_and_never_shrunk() {
    let (draft, piece) = rectangle(30.0, 30.0);
    let printed = piece_to_pdf(&draft, piece, A4).expect("a piece bigger than the paper tiles");
    let laid = &printed.piles[0];
    assert_eq!(laid.grid, [2, 2]);
    assert_eq!(laid.sheets, 4);
    assert_eq!(laid.blank(), 0, "a rectangle fills every cell of its box");
    let file = String::from_utf8_lossy(&printed.bytes).into_owned();
    assert_eq!(file.matches("/Type /Page ").count(), 4);
    assert!(file.contains("/Count 4"), "{file}");
    // Four sheets is a catalogue, a page tree, a font, and two objects each.
    assert!(file.contains("xref\n0 12\n"), "{file}");
}

/// The one claim a person taping two sheets together is trusting: a place both
/// sheets carry is the same place on the pattern, so lapping them by one step
/// lands the two drawings on each other.
///
/// Measured the way the paper is assembled — the same node of the cut line read
/// off two different pages, and the distance between the two readings compared
/// with the step the grid advances by — and not by asking the grid to agree
/// with itself.
#[test]
fn a_place_two_sheets_share_lands_on_the_same_point_of_the_pattern() {
    let (draft, piece) = rectangle(40.0, 10.0);
    let printed = piece_to_pdf(&draft, piece, A4).expect("a wide piece tiles");
    assert_eq!(printed.piles[0].grid, [3, 1]);
    let pages = streams_of(&printed.bytes);
    let step = points(cell(A4)[0] - OVERLAP);
    let mut worst_um = 0.0_f64;
    for pair in pages.windows(2) {
        let (left, right) = (path_of(&pair[0]), path_of(&pair[1]));
        assert_eq!(left.len(), right.len(), "every sheet carries every node");
        for (rank, node) in left.iter().enumerate() {
            let off = (node[0] - right[rank][0] - step).abs();
            worst_um = worst_um.max(off * 25_400.0 / 72.0);
        }
    }
    // Two decimals of a point is 3.5 um, so a place written on two sheets
    // carries half of that from each of them.
    assert!(worst_um < 4.0, "{worst_um} um out between two sheets");
    eprintln!("worst node across a joint: {worst_um:.3} um");
}

/// The marks a person matches the two sheets by are on the band both of them
/// carry, at the same place on the pattern, and under the same name.
#[test]
fn the_marks_of_a_joint_are_on_both_of_its_sheets_and_one_step_apart() {
    let (draft, piece) = rectangle(40.0, 10.0);
    let printed = piece_to_pdf(&draft, piece, A4).expect("a wide piece tiles");
    let pages = streams_of(&printed.bytes);
    // A joint is named after the two sheets it holds together, so the one
    // between the first two is C1-2, drawn on the right band of one and the
    // left band of the next.
    let first = marks(&pages[0], "C1-2");
    let second = marks(&pages[1], "C1-2");
    assert_eq!(
        first.len(),
        3,
        "a joint is marked three times down its band"
    );
    assert_eq!(second.len(), 3);
    let step = points(cell(A4)[0] - OVERLAP);
    for (rank, at) in first.iter().enumerate() {
        assert!((at - second[rank] - step).abs() < 0.02, "{at}");
    }
    assert!(marks(&pages[0], "C2-3").is_empty(), "no joint of its own");
    assert_eq!(marks(&pages[1], "C2-3").len(), 3);
}

/// A joint's name belongs to its two sheets and to no others.
///
/// The case one row cannot show, and the one that matters: a name taken from
/// the grid's column or row is worn by every sheet of that column or row, so on
/// two columns by two rows all four would say the same thing and a person
/// holding two of them could tape the wrong pair. Measured on a square grid,
/// where each of the four joints has exactly two sheets.
#[test]
fn a_joints_name_is_on_its_two_sheets_and_on_no_others() {
    let (draft, piece) = rectangle(30.0, 30.0);
    let printed = piece_to_pdf(&draft, piece, A4).expect("a piece bigger than the paper tiles");
    let pages = streams_of(&printed.bytes);
    // Sheets 1 2 / 3 4: two joints down the columns, two across the rows.
    for (joint, sheets) in [
        ("C1-2", [0, 1]),
        ("C3-4", [2, 3]),
        ("F1-3", [0, 2]),
        ("F2-4", [1, 3]),
    ] {
        for (rank, page) in pages.iter().enumerate() {
            let found = marks(page, joint).len();
            let want = usize::from(sheets.contains(&rank)) * 3;
            assert_eq!(found, want, "{joint} on sheet {} of 4", rank + 1);
        }
    }
}

/// A sheet says to cut only where there is another sheet to lap onto. The outer
/// edge of a piece is where its own cut line runs, and a dashed line inviting a
/// person to cut there invites cutting the pattern away.
#[test]
fn a_sheet_says_to_cut_only_where_another_sheet_meets_it() {
    let (draft, piece) = rectangle(30.0, 30.0);
    let printed = piece_to_pdf(&draft, piece, A4).expect("a piece bigger than the paper tiles");
    let pages = streams_of(&printed.bytes);
    // Two columns by two rows: every sheet has exactly two neighbours, the
    // corner sheets of a square grid having one of each.
    for page in &pages {
        assert_eq!(cuts(page), 2, "{page}");
    }
    let first = literal("Hojas vecinas: abajo 3 · derecha 2.");
    assert!(pages[0].contains(&first), "{}", pages[0]);
    let last = literal("Hojas vecinas: arriba 2 · izquierda 3.");
    assert!(pages[3].contains(&last), "{}", pages[3]);
}

/// Every sheet carries the square, not just the first: a printer that rescales
/// one tray of paper and not another is a real thing, and the person finds out
/// with a ruler or at the cutting table.
#[test]
fn every_sheet_carries_the_square_and_says_where_it_belongs() {
    let (draft, piece) = rectangle(30.0, 30.0);
    let printed = piece_to_pdf(&draft, piece, A4).expect("a piece bigger than the paper tiles");
    let pages = streams_of(&printed.bytes);
    for (rank, page) in pages.iter().enumerate() {
        assert!(page.contains(" re S"), "sheet {rank} has no square");
        // With the order in it. A tiled sheet also tells a person to cut along
        // the band this square sits in, so a legend that gave only its length
        // would have the proof of scale in the bin before it was ever read.
        let square = literal("El cuadrado mide 5 cm de lado: mídelo antes de recortarlo.");
        assert!(page.contains(&square), "{page}");
        assert!(
            page.contains("La pieza entera mide 30.0 cm de ancho"),
            "{page}"
        );
        let said = format!("hoja {} de 4", rank + 1);
        assert!(
            page.contains(&said),
            "sheet {rank} does not say which it is"
        );
    }
}

/// The cells a concave piece leaves empty are not printed, and the sheets that
/// are keep their numbering unbroken, so that sheet four is the fourth sheet
/// out of the printer and not the fourth cell of a grid.
#[test]
fn the_blank_cells_of_a_concave_piece_are_not_printed() {
    let (draft, piece) = ell();
    let printed = piece_to_pdf(&draft, piece, A4).expect("an L tiles");
    let laid = &printed.piles[0];
    assert_eq!(laid.grid, [2, 2]);
    assert_eq!(laid.sheets, 3);
    assert_eq!(laid.blank(), 1);
    let pages = streams_of(&printed.bytes);
    assert_eq!(pages.len(), 3);
    for (rank, page) in pages.iter().enumerate() {
        let said = format!("hoja {} de 3", rank + 1);
        assert!(page.contains(&said), "{page}");
    }
}

/// A tiled piece is as deterministic as a single sheet: the same document
/// writes the same forty sheets, or the file it wrote yesterday cannot be
/// reprinted.
#[test]
fn the_same_tiled_piece_writes_the_same_sheets_twice() {
    let (draft, piece) = rectangle(30.0, 30.0);
    let once = piece_to_pdf(&draft, piece, A4).expect("it tiles");
    let again = piece_to_pdf(&draft, piece, A4).expect("it tiles");
    assert_eq!(once, again);
}

/// A joint's name fits inside the band the joint is, which is the band's whole
/// job: it is the only paper both sheets carry.
///
/// A name that runs out of it is a name the neighbour's paper covers part of
/// once the sheets are lapped, and it stands on the paper the piece's own words
/// are laid out on, where a cutting instruction can be printed over it. Either
/// way a person reads `C17-1` for `C17-18`, tapes the wrong pair and finds out
/// at the cutting table.
///
/// Measured on a pile of twenty sheets, because a joint between two sheets of
/// one digit fits wherever it is put and the names that do not are the ones
/// with two.
#[test]
fn every_name_of_a_joint_fits_inside_the_band_it_names() {
    let (draft, piece) = rectangle(60.0, 100.0);
    let printed = piece_to_pdf(&draft, piece, A4).expect("a long piece tiles");
    assert_eq!(printed.piles[0].grid, [4, 5]);
    let [wide, _] = cell(A4);
    let (half, left) = (points(OVERLAP / 2.0), points(MARGIN + OVERLAP / 2.0));
    let right = points(MARGIN + wide - OVERLAP / 2.0);
    let mut seen = 0;
    for page in &streams_of(&printed.bytes) {
        for (at, name) in column_joints(page) {
            // Which band a name stands in is the one it is nearer: a sheet of
            // the middle columns laps one on either side.
            let middle = if (at - left).abs() < (at - right).abs() {
                left
            } else {
                right
            };
            // Measured the way the sheet lays type out, with the widths of the
            // font it names rather than with so much per character.
            let room = points(metric::wide(CAPTION, &name));
            assert!(at >= middle - half, "«{name}» opens outside its band");
            assert!(at + room <= middle + half, "«{name}» runs out of its band");
            seen += 1;
        }
    }
    assert!(seen >= 40, "{seen} column joints on twenty sheets");
}

/// Every name of a joint between two columns on one sheet, and where it opens
/// across the page, in points.
///
/// Read as a program would: the one word of the line that is a parenthesised
/// `C`, two whole numbers and the hyphen between them.
fn column_joints(stream: &str) -> Vec<(f64, String)> {
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    stream
        .lines()
        .filter_map(|line| {
            let word: Vec<&str> = line.split_whitespace().collect();
            let shown = word.get(7)?.strip_prefix("(C")?.strip_suffix(')')?;
            let (low, high) = shown.split_once('-')?;
            if !digits(low) || !digits(high) {
                return None;
            }
            Some((word[4].parse().ok()?, format!("C{shown}")))
        })
        .collect()
}

/// Where across the page each mark of one joint is written, in points.
fn marks(stream: &str, joint: &str) -> Vec<f64> {
    let named = format!("({joint}) Tj");
    stream
        .lines()
        .filter(|line| line.contains(&named))
        .filter_map(|line| line.split_whitespace().nth(4)?.parse().ok())
        .collect()
}

/// How many lines to cut on the sheet carries.
///
/// Counted inside the one dashed block of the stream, because the crosses and
/// the piece are drawn with the same verbs and only the dashes say which line a
/// person is meant to put scissors on.
fn cuts(stream: &str) -> usize {
    let dashed = stream
        .split_once("] 0 d\n")
        .expect("a tiled sheet has a line to cut on")
        .1;
    dashed
        .split_once("\nQ\n")
        .expect("the dashes are turned off again")
        .0
        .lines()
        .filter(|line| line.ends_with(" l S"))
        .count()
}
