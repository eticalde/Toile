use super::super::units::{MARGIN, box_of, centimetres};
use super::pack::Placed;
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

/// The most sheets one pile may be laid across.
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

/// One piece of a pile, on the plane the taped sheets make.
#[derive(Debug)]
pub(super) struct Cloth {
    /// Its outline on the plane, in millimetres.
    cut: Vec<[f64; 2]>,
    /// Where its every other place reaches that plane.
    plane: Plane,
    /// Where the box around it opens on the plane, in millimetres.
    place: [f64; 2],
    /// What it measures, in millimetres.
    size: [f64; 2],
}

impl Cloth {
    /// Its outline on the plane, in millimetres.
    pub(super) fn cut(&self) -> &[[f64; 2]] {
        &self.cut
    }

    /// One place of the piece on the plane, in millimetres.
    pub(super) fn onto(&self, at: [f64; 2]) -> [f64; 2] {
        self.plane.onto(at)
    }

    /// Where the box around it opens on the plane, in millimetres.
    pub(super) fn place(&self) -> [f64; 2] {
        self.place
    }

    /// What it measures, in millimetres.
    pub(super) fn size(&self) -> [f64; 2] {
        self.size
    }
}

/// Which sheets a pile needs, and where each of its pieces sits on them.
#[derive(Debug)]
pub(super) struct Grid {
    /// Every piece of the pile, in the order the file inks them.
    cloths: Vec<Cloth>,
    /// What the whole pile measures, in millimetres.
    size: [f64; 2],
    /// How many columns and rows of paper it is laid across.
    counts: [usize; 2],
    /// What each cell of the grid became, in reading order: the number of the
    /// sheet that carries it, or nothing where no cloth reaches the cell.
    sheets: Vec<Option<usize>>,
    /// How much of one sheet may be drawn on, in millimetres.
    cell: [f64; 2],
    /// How far one cell starts from the next, in millimetres.
    step: [f64; 2],
}

impl Grid {
    /// The grid a pile of placed pieces is laid across, on this paper.
    ///
    /// The paper left over once the columns are counted is shared out across
    /// them and kept off the rows, because a draft traces downward from the
    /// waist: a pile hangs from the top margin of its first row, where its own
    /// first line is.
    ///
    /// # Errors
    /// `SheetError::TooMany` when the pile asks for more paper than a ream.
    pub(super) fn new(paper: Paper, laid: &[Placed]) -> Result<Grid, SheetError> {
        debug_assert!(!laid.is_empty(), "a pile is opened by a piece");
        let (low, high) = box_of(laid.iter().flat_map(|piece| {
            let [x, y] = piece.place;
            [[x, y], [x + piece.size[0], y + piece.size[1]]]
        }));
        let size = [high[0] - low[0], high[1] - low[1]];
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
        let spare = (counts[0] - 1) as f64 * step[0] + cell[0] - size[0];
        let cloths: Vec<Cloth> = laid
            .iter()
            .map(|piece| laid_out(piece, low, spare / 2.0))
            .collect();
        let sheets = walked(&cloths, counts, cell, step);
        Ok(Grid {
            cloths,
            size,
            counts,
            sheets,
            cell,
            step,
        })
    }

    /// Every piece of the pile, in the order the file inks them.
    pub(super) fn cloths(&self) -> &[Cloth] {
        &self.cloths
    }

    /// What the whole pile measures, in millimetres.
    pub(super) fn size(&self) -> [f64; 2] {
        self.size
    }

    /// How many columns and rows the pile is laid across, blank cells and all.
    pub(super) fn counts(&self) -> [usize; 2] {
        self.counts
    }

    /// How many sheets of paper the pile prints.
    pub(super) fn sheets(&self) -> usize {
        self.sheets.iter().flatten().count()
    }

    /// Whether every sheet of the pile carries a part of every piece on it.
    ///
    /// The one thing that makes a pile one thing: a stack whose sheets are all
    /// the same pieces can be described once, on every sheet of it and in the
    /// summary, and a person holding any sheet of it is holding all of them.
    pub(super) fn one_pile(&self) -> bool {
        self.printed().all(|corner| {
            self.cloths
                .iter()
                .all(|cloth| lands::on(&cloth.cut, corner, self.cell))
        })
    }

    /// Where one cell's drawing starts on the tiled plane, in millimetres.
    pub(super) fn corner(&self, [col, row]: [usize; 2]) -> [f64; 2] {
        [col as f64 * self.step[0], row as f64 * self.step[1]]
    }

    /// Every sheet the pile prints, in the order they are printed.
    ///
    /// Reading order, left to right and top to bottom, which is the order the
    /// sheets come out of the printer and the order they are laid on the table.
    pub(super) fn places(&self) -> Vec<Place> {
        let total = self.sheets();
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

    /// The corner of every cell the pile prints, in reading order.
    fn printed(&self) -> impl Iterator<Item = [f64; 2]> {
        (0..self.counts[1]).flat_map(move |row| {
            (0..self.counts[0])
                .filter_map(move |col| self.at(col, row).map(|_| self.corner([col, row])))
        })
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

/// One placed piece as the plane carries it, with the spare paper shared in.
fn laid_out(piece: &Placed, low: [f64; 2], spare: f64) -> Cloth {
    let place = [piece.place[0] - low[0] + spare, piece.place[1] - low[1]];
    let plane = Plane::new(piece.low, place);
    let cut: Vec<[f64; 2]> = piece.outline.iter().map(|&at| plane.onto(at)).collect();
    Cloth {
        cut,
        plane,
        place,
        size: piece.size,
    }
}

/// How many sheets one axis of a pile takes.
///
/// Each sheet after the first advances by a step rather than by a whole cell,
/// because the band the two share is drawn twice and moves the pile forward
/// not at all.
fn across(size: f64, cell: f64, step: f64) -> usize {
    if size <= cell {
        return 1;
    }
    1 + (((size - cell) / step) - SLACK).ceil() as usize
}

/// Which cell became which sheet, in reading order.
fn walked(
    cloths: &[Cloth],
    counts: [usize; 2],
    cell: [f64; 2],
    step: [f64; 2],
) -> Vec<Option<usize>> {
    let mut sheets = Vec::with_capacity(counts[0] * counts[1]);
    let mut printed = 0;
    for row in 0..counts[1] {
        for col in 0..counts[0] {
            let corner = [col as f64 * step[0], row as f64 * step[1]];
            let lands = cloths
                .iter()
                .any(|cloth| lands::on(&cloth.cut, corner, cell));
            sheets.push(lands.then(|| {
                printed += 1;
                printed
            }));
        }
    }
    sheets
}

#[cfg(test)]
mod tests;
