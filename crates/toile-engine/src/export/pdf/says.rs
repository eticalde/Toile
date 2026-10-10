/// The one promise the whole layout is for, read off the page.
mod over;

use super::super::metric;
use super::super::units::CAPTION;
use super::grid::OVERLAP;
use super::paper::points;
use super::read::{anchor_of, clip_of, cuts_of, shown_of, streams_of, whole_of};
use super::text::literal;
use super::{A4, to_pdf};
use crate::draft::{Doc, Draft, MeasureSet, Piece, Point, Winding};

/// One rectangular piece, with everything its author said about cutting it out.
struct Told {
    /// What the pattern calls it.
    name: &'static str,
    /// Its sides, in centimetres.
    size: [f64; 2],
    /// The letter its label shows.
    letter: &'static str,
    /// How many of it the garment takes.
    quantity: u32,
    /// Its allowance in centimetres, or nothing where it is cut net.
    allowance: Option<f64>,
    /// The lines of its label, as the owner's own file carries them: the name
    /// in bold, then a phrase.
    labels: [&'static str; 2],
}

/// A product of rectangles carrying what the owner's file declares about them.
fn laid(told: &[Told]) -> Draft {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
    for piece in told {
        let [wide, tall] = piece.size;
        let corners = [[0.0, 0.0], [wide, 0.0], [wide, tall], [0.0, tall]];
        let points: Vec<_> = corners
            .into_iter()
            .map(|[x, y]| doc.points.insert(Point::at(x, y)))
            .collect();
        let mut held = Piece::polygon(piece.name, points, Winding::Cw);
        held.letter = Some(piece.letter.to_owned());
        held.quantity = piece.quantity;
        held.seam_allowance = piece.allowance;
        held.labels = piece.labels.iter().map(|&line| line.to_owned()).collect();
        doc.pieces.insert(held);
    }
    Draft::from_doc(doc).expect("rectangles resolve")
}

/// The owner's front panel and the strip that carries his belt loops: the two
/// extremes of his file, one cut in pairs with an allowance and one cut net.
///
/// Sized to share one sheet of A4, with the strip to the right of the panel,
/// which is also where the room for the words runs out.
fn both() -> Draft {
    laid(&[
        Told {
            name: "DELANTERO",
            size: [14.0, 8.0],
            letter: "A",
            quantity: 2,
            allowance: Some(1.5),
            labels: ["DELANTERO", "cortar 2 espejadas"],
        },
        Told {
            name: "TIRA PASACINTOS",
            size: [3.0, 8.0],
            letter: "F",
            quantity: 1,
            allowance: None,
            labels: ["TIRA PASACINTOS", "cortar 1 - doblar en tercios"],
        },
    ])
}

/// A panel bigger than the paper, so that what it says has to survive the
/// tiling as well as the printing.
fn tiled() -> Draft {
    laid(&[Told {
        name: "TRASERO + CANESU",
        size: [30.0, 30.0],
        letter: "B",
        quantity: 2,
        allowance: Some(1.5),
        labels: ["TRASERO + CANESU", "cortar 2 - separar canesu"],
    }])
}

/// What the whole thing is for: the sheet says the letter, the name, how many
/// to cut, what the allowance is and which line it is measured from, and then
/// the lines the owner wrote on his own pattern.
///
/// Read with the broken lines put back together, because the strip is three
/// centimetres wide and its own sentences are six: what the decision promises
/// is that every word reaches the paper, not that it reaches it on one row.
#[test]
fn every_piece_says_the_five_things_a_cutter_has_to_read() {
    let printed = to_pdf(&both(), A4).expect("both pieces print");
    assert_eq!(printed.sheets(), 1, "the two share one sheet");
    let page = whole_of(&streams_of(&printed.bytes)[0]);
    for said in [
        "A · «DELANTERO»",
        "14.0 × 8.0 cm",
        "Cortar 2 · margen 1.5 cm por fuera del contorno",
        "DELANTERO",
        "cortar 2 espejadas",
        "F · «TIRA PASACINTOS»",
        "3.0 × 8.0 cm",
        "Cortar 1 · sin margen: corta por el contorno",
        "TIRA PASACINTOS",
        "cortar 1 - doblar en tercios",
    ] {
        assert!(page.contains(said), "«{said}» is on no sheet");
    }
    assert_eq!(printed.piles[0].unsaid, 0, "both pieces found room");
}

/// The silence that cost the owner two trips to his notes while he cut these
/// jeans: a net piece says it is net, out loud, on the sheet.
///
/// And says it in words no piece with an allowance uses, so a person who has
/// both sheets in front of him is not reading the same sentence twice.
#[test]
fn a_net_piece_says_so_rather_than_leaving_the_question_open() {
    let net = laid(&[Told {
        name: "TIRA CADENA",
        size: [12.0, 6.0],
        letter: "I",
        quantity: 1,
        allowance: None,
        labels: ["TIRA CADENA", "al bies - repetir"],
    }]);
    let printed = to_pdf(&net, A4).expect("the strip prints");
    let page = &streams_of(&printed.bytes)[0];
    assert!(
        page.contains(&literal("Cortar 1 · sin margen: corta por el contorno")),
        "{page}"
    );
    assert!(!page.contains(&literal("por fuera del contorno")), "{page}");
}

/// Every sheet of a tiled piece carries what the piece says about being cut
/// out, and not just the first.
///
/// A person prints eighteen sheets, trims them and tapes them, and whichever
/// corner of the pattern he is holding has to answer the question. The legend
/// cannot: it sits in the band below the line the same sheet tells him to cut
/// along, so by the time the panel exists the legend does not.
#[test]
fn every_sheet_of_a_tiled_piece_says_how_the_piece_is_cut_out() {
    let printed = to_pdf(&tiled(), A4).expect("the panel tiles");
    let pages = streams_of(&printed.bytes);
    assert_eq!(pages.len(), 4, "two columns by two rows");
    for (rank, page) in pages.iter().enumerate() {
        for said in [
            "B · «TRASERO + CANESU»",
            "Cortar 2 · margen 1.5 cm por fuera del contorno",
            "cortar 2 - separar canesu",
        ] {
            assert!(
                page.contains(&literal(said)),
                "sheet {} of 4 does not say «{said}»",
                rank + 1
            );
        }
    }
}

/// And carries it inside the clip, which is the only part of the sheet that is
/// still there once the bands are trimmed away.
///
/// The bytes carrying a line is not the paper showing it: a line set past the
/// cell is written into the file and clipped off the page, so `contains` passes
/// and the cutter reads nothing. That is how fifteen of the owner's sixty-four
/// node names reached no sheet of his file.
#[test]
fn what_a_piece_says_is_set_inside_the_clip_and_not_merely_in_the_bytes() {
    let printed = to_pdf(&tiled(), A4).expect("the panel tiles");
    for (rank, page) in streams_of(&printed.bytes).iter().enumerate() {
        let [left, low, wide, tall] = clip_of(page);
        for said in [
            "B · «TRASERO + CANESU»",
            "30.0 × 30.0 cm",
            "Cortar 2 · margen 1.5 cm por fuera del contorno",
            "TRASERO + CANESU",
            "cortar 2 - separar canesu",
        ] {
            let [x, y] = anchor_of(page, &literal(said));
            assert!(
                x >= left && x <= left + wide && y >= low && y <= low + tall,
                "«{said}» is set at {x},{y} on sheet {}, outside the clip {left},{low} \
                 {wide}×{tall}",
                rank + 1
            );
        }
    }
}

/// And clear of the bands this sheet laps under its neighbours, which are the
/// one part of it another sheet's paper ends up covering — and where the mark
/// that names the joint is.
///
/// The last sheet of a two by two pile laps a neighbour above it and one to its
/// left, so one page shows both insets. The first sheet laps nothing on those
/// two sides, and keeps the room: it sets the same handle up in the paper the
/// last sheet has to leave clear, which is what says the inset is the
/// neighbour's and not a margin paid on every sheet.
#[test]
fn what_a_piece_says_is_kept_out_of_the_bands_the_sheets_lap_under() {
    let printed = to_pdf(&tiled(), A4).expect("the panel tiles");
    let pages = streams_of(&printed.bytes);
    let handle = literal("B · «TRASERO + CANESU»");
    let band = points(OVERLAP);
    let [left, low, _, tall] = clip_of(&pages[3]);
    let [x, y] = anchor_of(&pages[3], &handle);
    assert!(x >= left + band, "{x} is in the band down the left");
    // The page measures upward, so clear of the band along the top of the cell
    // is at or below the top of the clip less the band's own width.
    assert!(y <= low + tall - band, "{y} is in the band along the top");
    let [_, low, _, tall] = clip_of(&pages[0]);
    let [_, higher] = anchor_of(&pages[0], &handle);
    assert!(higher > low + tall - band, "{higher} is inset for nothing");
}

/// A piece narrower than its own sentences keeps them anyway: they are broken
/// to its width, and not one of them starts on its neighbour's paper.
///
/// This is the case the sheet used to answer the other way round. The strip is
/// three centimetres wide against a cutting instruction of six, and the block
/// was pulled left until the longest line fitted — which put the whole of it
/// across the panel beside it, where the paper no longer said which piece the
/// `Cortar 1` belonged to. Measured the way the scissors meet it: every line
/// read off the page against the strip's own cut line and against the panel's.
#[test]
fn a_piece_narrower_than_its_own_words_keeps_them_and_keeps_them_its_own() {
    let printed = to_pdf(&both(), A4).expect("both pieces print");
    let page = &streams_of(&printed.bytes)[0];
    let [left, _, wide, _] = clip_of(page);
    let cuts = cuts_of(page);
    let opens = |rank: usize| {
        cuts[rank]
            .iter()
            .map(|at| at[0])
            .fold(f64::INFINITY, f64::min)
    };
    let (panel, strip) = (opens(0), opens(1));
    let said = "Cortar 1 · sin margen: corta por el contorno";
    assert!(
        whole_of(page).contains(said),
        "the strip lost its instruction"
    );
    let mut broken = 0;
    for line in block_of(page, "F") {
        let room = points(metric::wide(CAPTION, &line.1));
        assert!(line.0 >= strip, "«{}» opens on the panel", line.1);
        assert!(line.0 + room <= left + wide, "«{}» runs off", line.1);
        broken += 1;
    }
    assert!(
        broken > 4,
        "{broken} lines: the strip's words did not break"
    );
    assert!(strip > panel, "the strip is the piece on the right");
}

/// Every line of one piece's block, with where it opens across the page: the
/// lines from the one that starts with the piece's own letter to the end of
/// the sheet's own clip.
fn block_of(stream: &str, letter: &str) -> Vec<(f64, String)> {
    let opened = format!("({letter} ");
    stream
        .split_once("\nQ\n")
        .map_or(stream, |held| held.0)
        .lines()
        .skip_while(|line| !line.contains(&opened))
        .filter_map(|line| {
            let word: Vec<&str> = line.split_whitespace().collect();
            let shown = shown_of(line).pop()?;
            Some((word[4].parse().ok()?, shown))
        })
        .collect()
}
