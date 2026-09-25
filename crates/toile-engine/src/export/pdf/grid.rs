use super::super::units::{MARGIN, box_of, centimetres, millimetres};
use super::paper::Paper;
use super::place::{Joins, Place};
use super::plane::Plane;
use super::{SheetError, lands, legend};

/// How wide the band two neighbouring sheets both carry, in millimetres.
///
/// Three things move between the two halves of a joint and the band has to
/// survive all of them at once. A printer places the image on the sheet to
/// within about a millimetre, so two sheets out of the same tray can miss each
/// other by two; a hand following a printed line with scissors wanders about
/// as much again. The mark a person lines the sheets up by therefore sits in
/// the middle of the band, and half a band has to be wider than that wander
/// plus the arm of the mark. The fourth reason is the roll of tape everybody
/// already owns: it is nineteen millimetres wide, and a lap narrower than the
/// tape sticks the joint to the table instead of to the other sheet.
///
/// Twenty is the first whole millimetre past that tape, and leaves ten either
/// side of the mark. It is paid out of every step, so on a piece already wider
/// than the paper it costs sheets — which is why the command prints the count.
pub(super) const OVERLAP: f64 = 20.0;

/// How little of a millimetre still counts as none, dividing a size by a step.
///
/// A nanometre of cloth is not another column of paper, and a size that comes
/// out of a formula lands on the wrong side of a whole step often enough.
const SLACK: f64 = 1.0e-9;

/// The most sheets one piece may be laid across.
///
/// A ream is five hundred sheets. A piece that asks for more than a ream is a
/// formula that slipped a decimal, and printing it would say so an hour and a
/// gigabyte later; the refusal says it now, in sheets.
const REAM: usize = 500;

/// How much of one sheet a pattern may be drawn on, in millimetres.
///
/// The sheet less its margins, and less the band that proves the scale. That
/// band is on every sheet and not just the first, so what it costs is a share
/// of every sheet: a printer that rescales one tray of paper and not another
/// is a real thing, and a person finds out with a ruler or not at all.
pub(super) fn cell(paper: Paper) -> [f64; 2] {
    [
        paper.width_mm - 2.0 * MARGIN,
        paper.height_mm - 2.0 * MARGIN - legend::BAND,
    ]
}

/// How far one sheet's drawing starts from the next one's, in millimetres.
fn step(paper: Paper) -> [f64; 2] {
    cell(paper).map(|side| side - OVERLAP)
}

/// Which sheets a piece needs, and where the cloth sits on them.
#[derive(Debug)]
pub(super) struct Grid {
    /// The cloth's outline on the tiled plane, in millimetres.
    cut: Vec<[f64; 2]>,
    /// The plane that outline and every other mark of the piece is laid on.
    plane: Plane,
    /// What the cloth measures, in millimetres.
    size: [f64; 2],
    /// How many columns and rows of paper it is laid across.
    counts: [usize; 2],
    /// What each cell of the grid became, in reading order: the number of the
    /// sheet that carries it, or nothing where no cloth reaches the cell.
    sheets: Vec<Option<usize>>,
    /// How far one cell starts from the next, in millimetres.
    step: [f64; 2],
}

impl Grid {
    /// The grid a piece of cloth is laid across, on this paper.
    ///
    /// # Errors
    /// `SheetError::TooMany` when the piece asks for more paper than a ream.
    pub(super) fn new(paper: Paper, cloth_cm: &[[f64; 2]]) -> Result<Grid, SheetError> {
        let (low, high) = box_of(cloth_cm.iter().copied());
        let size = millimetres([high[0] - low[0], high[1] - low[1]]);
        let (cell, step) = (cell(paper), step(paper));
        let counts = [0, 1].map(|axis| across(size[axis], cell[axis], step[axis]));
        let asked = counts[0].saturating_mul(counts[1]);
        if asked > REAM {
            return Err(SheetError::TooMany {
                sheets: asked,
                size_cm: centimetres(size),
                paper: paper.name,
            });
        }
        // How much paper is left over once the columns are counted, which is
        // what the plane shares out.
        let spare = (counts[0] - 1) as f64 * step[0] + cell[0] - size[0];
        let plane = Plane::new(low, spare);
        let cut: Vec<[f64; 2]> = cloth_cm.iter().map(|&at| plane.onto(at)).collect();
        let sheets = walked(&cut, counts, cell, step);
        Ok(Grid {
            cut,
            plane,
            size,
            counts,
            sheets,
            step,
        })
    }

    /// The cloth's outline on the tiled plane, in millimetres.
    pub(super) fn cut(&self) -> &[[f64; 2]] {
        &self.cut
    }

    /// One place of the piece on the tiled plane, in millimetres.
    pub(super) fn onto(&self, at: [f64; 2]) -> [f64; 2] {
        self.plane.onto(at)
    }

    /// What the whole piece measures, in millimetres.
    pub(super) fn size(&self) -> [f64; 2] {
        self.size
    }

    /// How many columns and rows the piece is laid across, blank cells and all.
    pub(super) fn counts(&self) -> [usize; 2] {
        self.counts
    }

    /// Where one cell's drawing starts on the tiled plane, in millimetres.
    pub(super) fn corner(&self, [col, row]: [usize; 2]) -> [f64; 2] {
        [col as f64 * self.step[0], row as f64 * self.step[1]]
    }

    /// Every sheet the piece prints, in the order they are printed.
    ///
    /// Reading order, left to right and top to bottom, which is the order the
    /// sheets come out of the printer and the order they are laid on the table.
    pub(super) fn places(&self) -> Vec<Place> {
        let total = self.sheets.iter().flatten().count();
        let mut out = Vec::with_capacity(total);
        for row in 0..self.counts[1] {
            for col in 0..self.counts[0] {
                let Some(number) = self.at(col, row) else {
                    continue;
                };
                out.push(Place {
                    number,
                    total,
                    cell: [col, row],
                    counts: self.counts,
                    joins: Joins {
                        above: row.checked_sub(1).and_then(|row| self.at(col, row)),
                        below: self.at(col, row + 1),
                        left: col.checked_sub(1).and_then(|col| self.at(col, row)),
                        right: self.at(col + 1, row),
                    },
                });
            }
        }
        out
    }

    /// The number of the sheet at one cell, or nothing where none was printed.
    fn at(&self, col: usize, row: usize) -> Option<usize> {
        if col >= self.counts[0] || row >= self.counts[1] {
            return None;
        }
        self.sheets
            .get(row * self.counts[0] + col)
            .copied()
            .flatten()
    }
}

/// How many sheets one axis of a piece takes.
///
/// Each sheet after the first advances by a step rather than by a whole cell,
/// because the band the two share is drawn twice and moves the piece forward
/// not at all.
fn across(size: f64, cell: f64, step: f64) -> usize {
    if size <= cell {
        return 1;
    }
    1 + (((size - cell) / step) - SLACK).ceil() as usize
}

/// Which cell became which sheet, in reading order.
fn walked(
    plane: &[[f64; 2]],
    counts: [usize; 2],
    cell: [f64; 2],
    step: [f64; 2],
) -> Vec<Option<usize>> {
    let mut sheets = Vec::with_capacity(counts[0] * counts[1]);
    let mut printed = 0;
    for row in 0..counts[1] {
        for col in 0..counts[0] {
            let corner = [col as f64 * step[0], row as f64 * step[1]];
            sheets.push(lands::on(plane, corner, cell).then(|| {
                printed += 1;
                printed
            }));
        }
    }
    sheets
}

#[cfg(test)]
mod tests {
    use super::super::A4;
    use super::super::tests::ELL;
    use super::*;

    /// A rectangle of cloth, in centimetres, hung at the document's origin.
    fn cloth(wide: f64, tall: f64) -> Vec<[f64; 2]> {
        vec![[0.0, 0.0], [wide, 0.0], [wide, tall], [0.0, tall]]
    }

    /// The sheet a piece that fits takes, and the one thing that must not have
    /// changed for it: it is still one sheet, and the cloth still sits on it
    /// where a sheet with no grid put it.
    #[test]
    fn a_piece_that_fits_is_one_sheet_and_the_grid_is_a_no_op() {
        let grid = Grid::new(A4, &cloth(12.0, 14.0)).expect("it fits");
        assert_eq!(grid.counts(), [1, 1]);
        let places = grid.places();
        assert_eq!(places.len(), 1);
        assert_eq!(
            places[0].joins,
            Joins {
                above: None,
                below: None,
                left: None,
                right: None
            }
        );
        let spare = (cell(A4)[0] - 120.0) / 2.0;
        let [x, y] = grid.cut()[0];
        assert!((x - spare).abs() < 1.0e-9, "{x} against {spare}");
        assert!(y.abs() < 1.0e-9, "{y}");
    }

    /// Each sheet after the first advances by a step and not by a cell, so the
    /// count is the one a person can check by laying the sheets out.
    #[test]
    fn a_piece_wider_than_the_paper_takes_a_column_for_every_step() {
        let cell = cell(A4)[0];
        let step = cell - OVERLAP;
        for (extra, want) in [(0.0, 1), (0.1, 2), (step, 2), (step + 0.1, 3)] {
            let wide = (cell + extra) / 10.0;
            let grid = Grid::new(A4, &cloth(wide, 1.0)).expect("it fits a ream");
            assert_eq!(grid.counts()[0], want, "{wide} cm");
        }
    }

    /// A size that lands a nanometre past a whole step is that step, and not
    /// another sheet of paper for a millionth of a millimetre of cloth.
    #[test]
    fn a_nanometre_past_a_whole_step_is_not_another_sheet() {
        let cell = cell(A4)[0];
        let step = cell - OVERLAP;
        let wide = (cell + step + 1.0e-10) / 10.0;
        let grid = Grid::new(A4, &cloth(wide, 1.0)).expect("it fits");
        assert_eq!(grid.counts()[0], 2);
    }

    /// The sheets are numbered in the order they are printed, and a cell with
    /// no cloth on it takes no number at all.
    #[test]
    fn the_numbers_run_in_reading_order_and_skip_the_blank_cells() {
        let grid = Grid::new(A4, &ELL).expect("it fits");
        assert_eq!(grid.counts(), [2, 2]);
        let places = grid.places();
        assert_eq!(places.len(), 3, "the hollow cell prints nothing");
        assert_eq!(places[0].cell, [0, 0]);
        assert_eq!(places[0].number, 1);
        assert_eq!(places[0].joins.right, None, "the sheet beside it is blank");
        assert_eq!(places[0].joins.below, Some(2));
        assert_eq!(places[1].cell, [0, 1]);
        assert_eq!(places[1].joins.above, Some(1));
        assert_eq!(places[2].cell, [1, 1]);
        assert_eq!(places[2].joins.left, Some(2));
        assert_eq!(places[2].joins.above, None);
    }

    /// A piece that asks for more paper than a ream is a formula that slipped a
    /// decimal, and it is said in sheets because that is what it would cost.
    #[test]
    fn a_piece_that_asks_for_more_than_a_ream_is_refused_in_sheets() {
        let refused = Grid::new(A4, &cloth(1_000.0, 1_000.0));
        let Err(SheetError::TooMany {
            sheets, size_cm, ..
        }) = refused
        else {
            panic!("ten metres of cloth each way is not a pattern: {refused:?}");
        };
        assert!(sheets > REAM, "{sheets}");
        assert!((size_cm[0] - 1_000.0).abs() < 1.0e-9, "{size_cm:?}");
        assert!((size_cm[1] - 1_000.0).abs() < 1.0e-9, "{size_cm:?}");
    }
}
