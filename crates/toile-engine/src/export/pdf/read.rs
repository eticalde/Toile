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
