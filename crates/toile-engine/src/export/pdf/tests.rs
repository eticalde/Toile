use super::paper::points_of_cm;
use super::text::literal;
use super::{A4, CARTA, Paper, SheetError, to_pdf};
use crate::draft::{Doc, Draft, MeasureSet, Piece, PieceKey, Point, Winding};

/// The sides of the test piece, in centimetres, chosen to fit on the smaller of
/// the two papers so that one fixture proves both.
const WIDE: f64 = 12.0;
const TALL: f64 = 14.0;

/// A piece whose every millimetre is known without resolving anything.
pub(super) fn polygon(corners: &[[f64; 2]]) -> (Draft, PieceKey) {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
    let points: Vec<_> = corners
        .iter()
        .map(|&[x, y]| doc.points.insert(Point::at(x, y)))
        .collect();
    let piece = doc
        .pieces
        .insert(Piece::polygon("Cuadro", points, Winding::Cw));
    (Draft::from_doc(doc).expect("a polygon resolves"), piece)
}

/// A rectangle, in centimetres, with a corner on the document's origin.
pub(super) fn rectangle(wide: f64, tall: f64) -> (Draft, PieceKey) {
    polygon(&[[0.0, 0.0], [wide, 0.0], [wide, tall], [0.0, tall]])
}

/// An L of cloth thirty centimetres each way, in centimetres.
///
/// Its upright is narrower than one sheet of A4 and its foot starts lower than
/// one, so the whole of the top right sheet of the two by two grid it takes
/// falls in the hollow. Everything about a blank cell is in these six places,
/// which is why they are here and not in whichever test asked first.
pub(super) const ELL: [[f64; 2]; 6] = [
    [0.0, 0.0],
    [13.0, 0.0],
    [13.0, 23.0],
    [30.0, 23.0],
    [30.0, 30.0],
    [0.0, 30.0],
];

/// That L as a piece of a document.
pub(super) fn ell() -> (Draft, PieceKey) {
    polygon(&ELL)
}

/// The bytes of a piece that takes exactly one sheet.
///
/// Every fixture in this file is one sheet on purpose: what one sheet says is
/// what it said before pieces were tiled at all, and a fixture that quietly
/// grew a second sheet would take the proof of scale with it.
fn printed(draft: &Draft, piece: PieceKey, paper: Paper) -> Vec<u8> {
    let printed = to_pdf(draft, piece, paper).expect("a rectangle prints");
    assert_eq!(printed.sheets, 1, "the fixture takes one sheet");
    assert_eq!(printed.grid, [1, 1]);
    printed.bytes
}

/// The one thing this whole file exists for: a length the document states in
/// centimetres comes off the page as the same length.
///
/// Measured the way a program that reads the file would measure it — parse the
/// path back, take the distance in points, divide by the transform — and not by
/// asking the transform to agree with itself.
#[test]
fn a_document_centimetre_is_the_same_length_on_the_page() {
    let (draft, piece) = rectangle(WIDE, TALL);
    let printed = printed(&draft, piece, A4);
    let drawn = path_of(&stream_of(&printed));
    let nodes = draft.cloth_cm(piece);
    assert_eq!(drawn.len(), nodes.len(), "every node reached the page");
    let mut worst_um = 0.0_f64;
    for rank in 0..nodes.len() {
        let next = (rank + 1) % nodes.len();
        let wanted_cm = step(nodes[rank], nodes[next]);
        let read_cm = step(drawn[rank], drawn[next]) / points_of_cm(1.0);
        worst_um = worst_um.max((read_cm - wanted_cm).abs() * 10_000.0);
    }
    // Two decimals of a point is 0.01 pt, which is 3.5 um, so an endpoint
    // carries up to half of that and a distance up to one whole.
    assert!(worst_um < 4.0, "{worst_um} um off true scale");
    eprintln!("worst side of the printed rectangle: {worst_um:.3} um off");
}

/// The page box is the real paper, so printing at 100 % is the identity and
/// the only thing left that can change the scale is the viewer's own
/// fit-to-page, which the sheet tells the reader in Spanish not to use.
#[test]
fn the_page_box_is_the_paper_and_not_a_shrunk_copy_of_it() {
    for paper in [A4, CARTA] {
        let (draft, piece) = rectangle(WIDE, TALL);
        let printed = printed(&draft, piece, paper);
        let [width, height] = media_box(&printed);
        let [want_width, want_height] = paper.points();
        assert!((width - want_width).abs() < 0.01, "{} {width}", paper.name);
        assert!(
            (height - want_height).abs() < 0.01,
            "{} {height}",
            paper.name
        );
    }
}

/// The square measures what the page says it measures, which is the only claim
/// on the sheet a person can check with a ruler and no software at all.
#[test]
fn the_calibration_square_measures_what_the_page_says() {
    let (draft, piece) = rectangle(WIDE, TALL);
    let printed = printed(&draft, piece, A4);
    let stream = stream_of(&printed);
    let [_, _, wide, tall] = square_of(&stream);
    let side_cm = wide / points_of_cm(1.0);
    let off_um = (side_cm - 5.0).abs() * 10_000.0;
    // The side is one number written to two decimals of a point, so it carries
    // half of 0.01 pt, which is under two micrometres.
    assert!(off_um < 2.0, "{side_cm} cm");
    assert!(
        (tall - wide).abs() < 1.0e-9,
        "{wide} by {tall} is no square"
    );
    let said = literal("El cuadrado mide 5 cm de lado.");
    assert!(stream.contains(&said), "the page does not say its own side");
    eprintln!("the printed calibration square: {off_um:.3} um off five centimetres");
}

/// The same document writes the same bytes. The other half of this proof runs
/// in a second process, which is the only way to catch an address or a clock
/// reaching the file.
#[test]
fn the_same_piece_writes_the_same_sheet_twice() {
    let (draft, piece) = rectangle(WIDE, TALL);
    let once = printed(&draft, piece, A4);
    let again = printed(&draft, piece, A4);
    assert_eq!(once, again);
}

/// Nothing a second run would write differently is in the file at all.
#[test]
fn nothing_on_the_sheet_comes_from_the_clock() {
    let (draft, piece) = rectangle(WIDE, TALL);
    let printed = String::from_utf8_lossy(&printed(&draft, piece, A4)).into_owned();
    for absent in ["/CreationDate", "/ModDate", "/Producer", "/Info", "/ID"] {
        assert!(!printed.contains(absent), "{absent} is in the sheet");
    }
}

#[test]
fn a_piece_that_resolves_to_no_contour_is_no_sheet() {
    let mut doc = Doc::new(MeasureSet::new("Etienne", [("cintura", 84.0)]));
    let points = vec![doc.points.insert(Point::at(0.0, 0.0))];
    let piece = doc
        .pieces
        .insert(Piece::polygon("Punto", points, Winding::Cw));
    let draft = Draft::from_doc(doc).expect("a one-point piece resolves");
    assert_eq!(to_pdf(&draft, piece, A4), Err(SheetError::Empty));
}

/// The cross-reference table is what a reader trusts before anything else: it
/// is read first, and an object one byte from where it says means the file is
/// refused whole rather than read as far as it goes.
#[test]
fn the_cross_reference_table_points_at_every_object() {
    let (draft, piece) = rectangle(WIDE, TALL);
    let printed = printed(&draft, piece, A4);
    let table = startxref(&printed);
    assert_eq!(&printed[table..table + 5], b"xref\n");
    let heading = table + 5;
    let opens = heading + until_newline(&printed, heading) + 1;
    let count: usize = ascii(&printed[heading..opens])
        .split_whitespace()
        .nth(1)
        .and_then(|word| word.parse().ok())
        .expect("the table says how many objects it covers");
    // The first entry is the head of the chain of free objects and names none.
    assert_eq!(&printed[opens..opens + 20], b"0000000000 65535 f\r\n");
    for id in 1..count {
        let at = opens + id * 20;
        let offset: usize = ascii(&printed[at..at + 10])
            .parse()
            .expect("an entry opens with ten digits of offset");
        let head = format!("{id} 0 obj");
        assert_eq!(
            &printed[offset..offset + head.len()],
            head.as_bytes(),
            "object {id} is not where the table says"
        );
    }
}

/// Where the trailer says the cross-reference table starts.
///
/// Read off the tail, because that is where a reader starts: it seeks to the
/// end, walks back to `startxref`, and goes where it points.
fn startxref(printed: &[u8]) -> usize {
    let tail = ascii(&printed[printed.len().saturating_sub(96)..]).to_owned();
    tail.rsplit_once("startxref\n")
        .and_then(|(_, rest)| rest.lines().next())
        .and_then(|line| line.parse().ok())
        .expect("the trailer says where the table starts")
}

/// How far the next end of line is from `at`.
fn until_newline(printed: &[u8], at: usize) -> usize {
    printed[at..]
        .iter()
        .position(|&byte| byte == b'\n')
        .expect("the line ends")
}

/// A run of the file's own bytes as text. Every byte the table and the trailer
/// are made of is printable by construction, so a run of them that is not is a
/// broken file and not a decoding to fall back from.
fn ascii(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).expect("the file's structure is written in ASCII")
}

/// The page's content stream, as the one reader of this file cares about it.
pub(super) fn stream_of(printed: &[u8]) -> String {
    let text = String::from_utf8_lossy(printed).into_owned();
    let opened = text
        .split_once("stream\n")
        .expect("the page has a content stream")
        .1;
    opened
        .split_once("endstream")
        .expect("the stream closes")
        .0
        .to_owned()
}

/// The first path of the stream, in the order it is drawn, in points.
pub(super) fn path_of(stream: &str) -> Vec<[f64; 2]> {
    stream
        .lines()
        .take_while(|line| *line != "h S")
        .filter_map(|line| {
            let mut word = line.split_whitespace();
            let x: f64 = word.next()?.parse().ok()?;
            let y: f64 = word.next()?.parse().ok()?;
            matches!(word.next(), Some("m" | "l")).then_some([x, y])
        })
        .collect()
}

/// The rectangle the stream strokes for calibration, in points.
pub(super) fn square_of(stream: &str) -> [f64; 4] {
    let line = stream
        .lines()
        .find(|line| line.ends_with(" re S"))
        .expect("the sheet carries a calibration square");
    let numbers: Vec<f64> = line
        .split_whitespace()
        .filter_map(|word| word.parse().ok())
        .collect();
    [numbers[0], numbers[1], numbers[2], numbers[3]]
}

/// The page's own size in points, as the file states it.
pub(super) fn media_box(printed: &[u8]) -> [f64; 2] {
    let text = String::from_utf8_lossy(printed).into_owned();
    let opened = text
        .split_once("/MediaBox [0 0 ")
        .expect("the page states a box")
        .1;
    let stated = opened.split_once(']').expect("the box closes").0;
    let numbers: Vec<f64> = stated
        .split_whitespace()
        .filter_map(|word| word.parse().ok())
        .collect();
    [numbers[0], numbers[1]]
}

/// The distance between two places, in whatever unit they are both in.
pub(super) fn step(from: [f64; 2], to: [f64; 2]) -> f64 {
    (to[0] - from[0]).hypot(to[1] - from[1])
}

/// The paper a sheet was asked for, so the two sizes are named once here.
#[test]
fn both_papers_are_named_on_the_sheet_they_print() {
    for paper in [A4, CARTA] {
        let (draft, piece) = rectangle(WIDE, TALL);
        let printed = printed(&draft, piece, paper);
        let stream = stream_of(&printed);
        let said = literal(&format!(
            "Papel {} · {} × {} mm.",
            paper.name, paper.width_mm, paper.height_mm
        ));
        assert!(stream.contains(&said), "{} is not named", paper.name);
        assert!(stream.contains(&literal("Pieza «Cuadro»")));
    }
}
