use std::collections::BTreeMap;

use super::super::drawn::Inked;
use super::grid::Grid;
use super::ink::Ink;
use super::pack::{self, Placed};
use super::paper::Paper;
use super::sheet::{Frame, content};
use super::{SheetError, file, named, objects};
use crate::draft::{Draft, PieceKey};

/// One piece of a print, as the paper took it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Laid {
    /// What the pattern calls the piece.
    pub name: String,
    /// How much of the piece besides its cut line reached the paper.
    pub inked: Inked,
    /// How many sheets it would have taken with paper to itself.
    ///
    /// What the print would have cost one piece per pile, so the command can
    /// say what sharing the paper saved. A waistband half a metre long and four
    /// centimetres tall takes three sheets of A4 with a plane to itself.
    pub alone: usize,
}

/// One stack of sheets inside a file: where it opens, how much paper it is, and
/// which pieces it carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pile {
    /// Which page of the file the first of its sheets is, counting from one.
    ///
    /// The number a print dialogue asks for, and the one the paper does not
    /// carry: a sheet says which sheet of its own pile it is, and one file
    /// holds every pile of the product.
    pub first_page: usize,
    /// How many sheets it takes.
    pub sheets: usize,
    /// How many columns and rows it was laid across, blank cells and all.
    pub grid: [usize; 2],
    /// Every piece on it, in the order the pattern draws them.
    ///
    /// More than one where the pieces were small enough to share the paper,
    /// and every sheet of the pile then carries a part of every one of them.
    pub pieces: Vec<Laid>,
}

impl Pile {
    /// How many cells of the grid carry no cloth and are therefore not printed.
    #[must_use]
    pub fn blank(&self) -> usize {
        self.grid[0] * self.grid[1] - self.sheets
    }

    /// The last page of the file the pile covers.
    #[must_use]
    pub fn last_page(&self) -> usize {
        self.first_page + self.sheets - 1
    }

    /// What the pile is called: its first piece, which is the handle every
    /// sheet of it prints.
    #[must_use]
    pub fn name(&self) -> &str {
        self.pieces.first().map_or("", |piece| piece.name.as_str())
    }
}

/// One piece a print left on the table, and what stopped it.
#[derive(Debug, Clone, PartialEq)]
pub struct Skipped {
    /// What the pattern calls the piece.
    pub name: String,
    /// What stopped it.
    pub why: SheetError,
}

/// A product no sheet of which could be laid out.
///
/// Carries every piece and not the first: a product whose pieces all fail is as
/// many separate things to fix as it has pieces, and a person handed one name
/// fixes one piece and prints nothing again.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("no piece of the pattern reached paper")]
pub struct NothingPrinted {
    /// Every piece of the product, and what stopped each one.
    pub left_out: Vec<Skipped>,
}

/// A print at true scale: the file, and what each piece of it became.
#[derive(Debug, Clone, PartialEq)]
pub struct Printed {
    /// The file, of one page per sheet.
    pub bytes: Vec<u8>,
    /// Each stack of sheets, in the order the file prints them.
    pub piles: Vec<Pile>,
    /// Each piece that reached no paper, and what stopped it.
    pub left_out: Vec<Skipped>,
}

impl Printed {
    /// How many sheets the file prints.
    #[must_use]
    pub fn sheets(&self) -> usize {
        self.piles.iter().map(|pile| pile.sheets).sum()
    }

    /// How many pieces reached the paper.
    #[must_use]
    pub fn pieces(&self) -> usize {
        self.piles.iter().map(|pile| pile.pieces.len()).sum()
    }

    /// How much paper the same pieces would have taken one pile each.
    #[must_use]
    pub fn alone(&self) -> usize {
        self.laid().map(|piece| piece.alone).sum()
    }

    /// How many sheets sharing the paper saved.
    ///
    /// A piece only ever joins a pile that takes no more sheets with it than
    /// without, so this is never negative and needs no signed answer.
    #[must_use]
    pub fn saved(&self) -> usize {
        self.alone().saturating_sub(self.sheets())
    }

    /// The names more than one piece of the file carries.
    ///
    /// What tells one loose sheet from another is the pieces it names, so two
    /// pieces under one name are two things nobody can separate on the table.
    /// Said and not refused: the paper is right, and what has to change is a
    /// name the person gave.
    #[must_use]
    pub fn twinned(&self) -> Vec<&str> {
        let mut counted: BTreeMap<&str, usize> = BTreeMap::new();
        for piece in self.laid() {
            *counted.entry(piece.name.as_str()).or_default() += 1;
        }
        counted
            .into_iter()
            .filter(|&(_, how_many)| how_many > 1)
            .map(|(name, _)| name)
            .collect()
    }

    /// How much of the whole print besides its cut lines reached the paper.
    ///
    /// Added up here because the counts are the engine's own reckoning and not
    /// the sheet's: a mark drawn on half a folded piece is inked twice and
    /// counted once, so the sum of the pieces is the sum a person would reach.
    #[must_use]
    pub fn inked(&self) -> Inked {
        self.laid().fold(Inked::default(), |mut all, piece| {
            all.lines += piece.inked.lines;
            all.notches += piece.inked.notches;
            all.names += piece.inked.names;
            all
        })
    }

    /// Every piece of every pile, in the order the file prints them.
    fn laid(&self) -> impl Iterator<Item = &Laid> {
        self.piles.iter().flat_map(|pile| pile.pieces.iter())
    }
}

/// A whole product as one file of sheets to print at true scale.
///
/// Every sheet of a pile together, and the piles in the order the pattern draws
/// their first piece. That order is the document's own, so it holds still: an
/// order taken from the sizes lays one product one way on A4 and another on
/// Carta — the owner's front pocket bag is one sheet of the first and two of
/// the second — and shuffles it again after any change of ease.
///
/// Pieces small enough to share a sheet do, on paper one of them needed anyway.
///
/// A piece that cannot be laid out is named and left out rather than taking the
/// rest down: the pieces that do reach paper are worth more than a refusal.
///
/// # Errors
/// `NothingPrinted` when no piece reached paper, naming what stopped each.
pub fn to_pdf(draft: &Draft, paper: Paper) -> Result<Printed, NothingPrinted> {
    let mut ready = Vec::new();
    let mut left_out = Vec::new();
    for piece in draft.doc().piece_keys() {
        match prepared(draft, piece, paper) {
            Ok(laid) => ready.push(laid),
            Err(why) => left_out.push(Skipped {
                name: named(draft, piece),
                why,
            }),
        }
    }
    let mut streams: Vec<String> = Vec::new();
    let mut piles = Vec::new();
    for laid in pack::piles(paper, ready) {
        let (own, mut pile) = written(draft, &laid, paper);
        pile.first_page = streams.len() + 1;
        piles.push(pile);
        streams.extend(own);
    }
    if streams.is_empty() {
        return Err(NothingPrinted { left_out });
    }
    Ok(Printed {
        bytes: file(&objects(paper, &streams)),
        piles,
        left_out,
    })
}

/// One piece of a product as a file of its own, for a person who re-cut one
/// panel and does not want the rest of the garment again.
///
/// A piece larger than the paper is tiled and never shrunk: a trouser leg is
/// most of a metre long and no sheet of paper is. It shares its paper with
/// nothing, because nothing else was asked for.
///
/// # Errors
/// `SheetError::Empty` when the piece resolves to no contour, and
/// `SheetError::TooMany` when it would take more paper than a ream.
pub fn piece_to_pdf(draft: &Draft, piece: PieceKey, paper: Paper) -> Result<Printed, SheetError> {
    let laid = prepared(draft, piece, paper)?;
    let (streams, pile) = written(draft, std::slice::from_ref(&laid), paper);
    Ok(Printed {
        bytes: file(&objects(paper, &streams)),
        piles: vec![pile],
        left_out: Vec::new(),
    })
}

/// One piece with a plane of its own, and what that plane costs.
///
/// Every piece is laid out alone before any of them shares, so the refusals are
/// about the piece and not about whichever neighbour it landed beside.
fn prepared(draft: &Draft, piece: PieceKey, paper: Paper) -> Result<Placed, SheetError> {
    let outline = draft.cloth_cm(piece);
    if outline.len() < 3 {
        return Err(SheetError::Empty);
    }
    let mut laid = Placed::new(piece, outline.to_vec());
    laid.alone = Grid::new(paper, std::slice::from_ref(&laid))?.sheets();
    Ok(laid)
}

/// Every sheet one pile takes, and the pile they make.
///
/// The pile opens on the first page, which is where a file of one piece puts
/// it; a product moves it to wherever the piles before it left off.
fn written(draft: &Draft, laid: &[Placed], paper: Paper) -> (Vec<String>, Pile) {
    let grid = Grid::new(paper, laid).expect("every piece joined a pile that was laid out");
    let inks: Vec<Ink> = laid
        .iter()
        .map(|piece| Ink::new(draft, piece.piece))
        .collect();
    // Every node of every piece is inside the box the grid is built from, so a
    // pile of pieces that resolve always lands on at least one sheet.
    let streams: Vec<String> = grid
        .places()
        .iter()
        .map(|place| {
            let frame = Frame::new(paper, grid.corner(place.cell));
            content(&frame, &grid, &inks, place)
        })
        .collect();
    let pieces = laid
        .iter()
        .zip(&inks)
        .map(|(piece, ink)| Laid {
            name: ink.name().to_owned(),
            inked: ink.inked(),
            alone: piece.alone,
        })
        .collect();
    let pile = Pile {
        first_page: 1,
        sheets: streams.len(),
        grid: grid.counts(),
        pieces,
    };
    (streams, pile)
}
