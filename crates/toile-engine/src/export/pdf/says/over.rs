use super::super::super::metric;
use super::super::read::{shown_of, streams_of};
use super::super::{A4, to_pdf};
use super::{both, tiled};

/// No two lines of type on a sheet are set over each other, whichever piece
/// each of them belongs to.
///
/// The one promise the whole layout is for, read off the page the way a reader
/// renders it: the node names of every piece and then the words of every piece,
/// each measured in the font the file names. A sheet carrying two pieces lays
/// the second one's block after the first one's is already on the paper, so
/// what the second has to keep off is both the names and that block.
#[test]
fn no_two_lines_of_type_on_a_sheet_are_set_over_each_other() {
    for draft in [both(), tiled()] {
        let printed = to_pdf(&draft, A4).expect("the pieces print");
        for (rank, page) in streams_of(&printed.bytes).iter().enumerate() {
            let set = boxes_of(page);
            for (one, first) in &set {
                for (two, second) in &set {
                    assert!(
                        std::ptr::eq(one, two) || !over(*one, *two),
                        "sheet {} sets «{first}» over «{second}»",
                        rank + 1
                    );
                }
            }
        }
    }
}

/// Every line of type one sheet sets inside its own clip, as the box it takes
/// on the page in points.
fn boxes_of(stream: &str) -> Vec<([f64; 4], String)> {
    stream
        .split_once("\nQ\n")
        .map_or(stream, |held| held.0)
        .lines()
        .filter_map(|line| {
            let word: Vec<&str> = line.split_whitespace().collect();
            if word.first() != Some(&"BT") {
                return None;
            }
            let shown = shown_of(line).pop()?;
            let size: f64 = word[2].parse().ok()?;
            let (at, up): (f64, f64) = (word[4].parse().ok()?, word[5].parse().ok()?);
            let box_of = [
                at,
                up - metric::BELOW * size,
                at + metric::wide(size, &shown),
                up + metric::ABOVE * size,
            ];
            Some((box_of, shown))
        })
        .collect()
}

/// Whether two boxes of the page share any ink at all.
fn over(one: [f64; 4], two: [f64; 4]) -> bool {
    one[0] < two[2] && two[0] < one[2] && one[1] < two[3] && two[1] < one[3]
}
