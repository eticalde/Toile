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

/// Every page's content stream, in the order the file writes them.
pub(super) fn streams_of(printed: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(printed)
        .split("\nstream\n")
        .skip(1)
        .map(|rest| {
            rest.split_once("endstream")
                .expect("the stream closes")
                .0
                .to_owned()
        })
        .collect()
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

/// Every closed path the stream strokes, in points: one per piece the sheet
/// carries.
///
/// A cut line is the only path the sheet closes before stroking it, so what
/// comes back is the outlines and none of the marks drawn inside them.
pub(super) fn cuts_of(stream: &str) -> Vec<Vec<[f64; 2]>> {
    let mut out = Vec::new();
    let mut path: Vec<[f64; 2]> = Vec::new();
    for line in stream.lines() {
        if line == "h S" {
            out.push(std::mem::take(&mut path));
            continue;
        }
        let word: Vec<&str> = line.split_whitespace().collect();
        let place = |x: &str, y: &str| Some([x.parse().ok()?, y.parse().ok()?]);
        match word.as_slice() {
            [x, y, "m"] => path = place(x, y).map(|at| vec![at]).unwrap_or_default(),
            [x, y, "l"] => path.extend(place(x, y)),
            _ => {}
        }
    }
    out
}

/// The rectangle the stream clips its pieces to, in points: where it opens,
/// then how wide and how tall it is.
pub(super) fn clip_of(stream: &str) -> [f64; 4] {
    let opened = stream.split_once(" re W n").expect("the sheet clips").0;
    let numbers: Vec<f64> = opened
        .rsplit('\n')
        .next()
        .expect("the clip is one line")
        .split_whitespace()
        .filter_map(|word| word.parse().ok())
        .collect();
    <[f64; 4]>::try_from(numbers).expect("a rectangle is four operands")
}

/// Where the sheet sets one line of type, in points.
///
/// Told the string as the page spells it rather than as a person reads it, so
/// that nothing here has to know how a reader's own strings are escaped.
pub(super) fn anchor_of(stream: &str, shown: &str) -> [f64; 2] {
    let shown = format!("{shown} Tj");
    let line = stream
        .lines()
        .find(|line| line.contains(&shown))
        .unwrap_or_else(|| panic!("{shown} is not on the sheet"));
    let word: Vec<&str> = line.split_whitespace().collect();
    let at = word
        .iter()
        .position(|&word| word == "Td")
        .expect("a line of type is placed before it is shown");
    [
        word[at - 2].parse().expect("the place across the page"),
        word[at - 1].parse().expect("the place up the page"),
    ]
}

/// Every line of type the sheet sets, in the order it sets them, in the
/// Spanish they spell.
pub(super) fn shown_of(stream: &str) -> Vec<String> {
    stream
        .lines()
        .filter_map(|line| {
            let opened = line.find('(')?;
            let closed = line.rfind(") Tj")?;
            Some(spelled(&line[opened + 1..closed]))
        })
        .collect()
}

/// What the sheet says with every line it broke put back together.
///
/// A block is broken only at a space, so joining the sheet's lines with one
/// space is how a person reading the paper puts the sentence back — and the
/// only way to ask whether a sentence survived the breaking rather than
/// whether it happened to fit on one row.
pub(super) fn whole_of(stream: &str) -> String {
    shown_of(stream).join(" ")
}

/// One of the page's own strings as the Spanish it stands for.
///
/// The way round the sheet's own escaping goes, because what a reader of the
/// file has is the escapes: anything measured on `\267` would be measuring
/// four characters where a printer sets one `·`. The codes above 160 are their
/// own code points, which is the whole of what the Spanish on these sheets
/// needs.
fn spelled(shown: &str) -> String {
    let mut out = String::new();
    let mut at = 0;
    while let Some(byte) = shown.as_bytes().get(at) {
        if *byte != b'\\' {
            out.push(char::from(*byte));
            at += 1;
            continue;
        }
        // Every escape the sheet writes is one backslashed punctuation mark or
        // exactly three octal digits, so a code is three characters or none.
        let escape = shown.get(at + 1..at + 4).unwrap_or_default();
        if let Some(character) = u32::from_str_radix(escape, 8).ok().and_then(char::from_u32) {
            out.push(character);
            at += 4;
        } else {
            out.extend(shown.as_bytes().get(at + 1).map(|&byte| char::from(byte)));
            at += 2;
        }
    }
    out
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
