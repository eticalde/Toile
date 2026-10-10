use std::fmt::Write;

use toile_engine::export::{self, NothingPrinted, Printed, Skipped};

use crate::file;

impl crate::App {
    /// Lays every piece of the pattern onto sheets to print at true scale.
    ///
    /// One file for the whole product, because printing a garment is one job at
    /// the printer and one stack of paper on the table. A piece that cannot be
    /// laid out is named and the rest are written anyway.
    ///
    /// The paper is the installation's, and the print says which it was: a file
    /// laid out for one size and printed on the other is asked for at 100 % by
    /// every sheet it holds, and no printer can grant that for a page taller
    /// than the paper it is fed.
    pub(super) fn print(&mut self) {
        let revision = self.session.revision();
        let paper = self.prefs.paper().sheet();
        let laid = self.session.draft().map(|held| export::to_pdf(held, paper));
        let printed = match laid {
            Some(Ok(printed)) => printed,
            Some(Err(why)) => {
                self.file.warn(refused(&why), revision);
                return;
            }
            None => {
                self.file
                    .warn("no hay ningún patrón que imprimir", revision);
                return;
            }
        };
        let Some(path) = file::pdf_target(self.file.stem()) else {
            return;
        };
        if let Err(why) = file::write_bytes(&path, &printed.bytes) {
            self.file.warn(why, revision);
            return;
        }
        // A file that went out short of a piece, short of a label, or with two
        // piles a person cannot separate, is reported as something wrong even
        // though it was written: they are about to cut a garment, and the panel
        // they cannot find is the one they will look for last.
        let said = wrote(&printed, paper.name);
        if printed.left_out.is_empty() && printed.twinned().is_empty() && printed.unsaid() == 0 {
            self.file.say(said, revision);
        } else {
            self.file.warn(said, revision);
        }
    }
}

/// What was written, what was left out of it, which sheets went out without
/// their piece's own words, and what a person will not be able to tell apart on
/// the table.
///
/// The paper is named here because the sheets themselves only say it once they
/// are printed: this is the one place the choice is confirmed while the ream is
/// still in the drawer.
///
/// And the labels are counted here because the owner prints from this button
/// and not from the terminal: the sheet that could not fit a piece's words
/// carries its outline and says nothing, so a count that only the command line
/// printed is a count he never sees.
fn wrote(printed: &Printed, paper: &str) -> String {
    let mut said = format!(
        "PDF a escala real · {} en {} de {paper}",
        plural(printed.pieces(), "pieza", "piezas"),
        plural(printed.sheets(), "hoja", "hojas")
    );
    if !printed.left_out.is_empty() {
        let _ = write!(
            said,
            " · sin imprimir: {}",
            quoted(&names(&printed.left_out))
        );
    }
    if printed.unsaid() > 0 {
        let _ = write!(
            said,
            " · {} sin sitio en su hoja: esa hoja lleva el contorno y no el rótulo",
            plural(printed.unsaid(), "rótulo de pieza", "rótulos de pieza")
        );
    }
    if !printed.twinned().is_empty() {
        let _ = write!(
            said,
            " · más de una pieza se llama {}",
            quoted(&printed.twinned())
        );
    }
    said
}

/// A count with the word that agrees with it: a print of one sheet is «1 hoja»,
/// and a studio does not read its own language wrong.
fn plural(how_many: usize, one: &str, more: &str) -> String {
    if how_many == 1 {
        format!("1 {one}")
    } else {
        format!("{how_many} {more}")
    }
}

/// The names of the pieces a print left on the table.
fn names(left_out: &[Skipped]) -> Vec<&str> {
    left_out.iter().map(|piece| piece.name.as_str()).collect()
}

/// A run of names as the notice sets them.
fn quoted(names: &[&str]) -> String {
    names
        .iter()
        .map(|name| format!("«{name}»"))
        .collect::<Vec<String>>()
        .join(", ")
}

/// Why nothing was written, naming every piece that stopped it.
fn refused(why: &NothingPrinted) -> String {
    if why.left_out.is_empty() {
        return "no hay ninguna pieza que imprimir".to_owned();
    }
    format!(
        "ninguna pieza llega al papel: revisa {}",
        quoted(&names(&why.left_out))
    )
}

#[cfg(test)]
mod tests {
    use toile_engine::draft::{Draft, block};
    use toile_engine::export::{Laid, Pile, SheetError};

    use super::*;
    use crate::config::Paper;

    /// A print of those piles and those refusals, with no bytes: what is under
    /// test is the words.
    fn print(piles: Vec<Pile>, left_out: Vec<Skipped>) -> Printed {
        Printed {
            bytes: Vec::new(),
            piles,
            left_out,
        }
    }

    /// One pile of one sheet carrying one piece, to say the paper with.
    fn pile(name: &str, first_page: usize) -> Pile {
        Pile {
            first_page,
            sheets: 1,
            grid: [1, 1],
            pieces: vec![Laid {
                name: name.to_owned(),
                inked: export::Inked::default(),
                alone: 1,
            }],
            unsaid: 0,
        }
    }

    /// The whole product in one file is what the button promises, so the notice
    /// counts the pieces and the sheets it actually wrote.
    #[test]
    fn the_notice_counts_the_pieces_and_the_sheets_of_the_file() {
        let said = wrote(
            &print(vec![pile("Delantero", 1), pile("Trasero", 2)], vec![]),
            "A4",
        );
        assert_eq!(said, "PDF a escala real · 2 piezas en 2 hojas de A4");
    }

    /// The paper the installation chose is the paper the notice names: it is
    /// what tells a person the stack in their hand will print at true scale on
    /// the ream they own.
    #[test]
    fn the_notice_names_the_paper_the_sheets_were_laid_out_on() {
        let carta = Paper::Carta.sheet();
        let said = wrote(&print(vec![pile("Pretina", 1)], vec![]), carta.name);
        assert_eq!(said, "PDF a escala real · 1 pieza en 1 hoja de Carta");
    }

    /// A piece that did not reach the paper is named in the notice, because the
    /// file was written and looks complete.
    #[test]
    fn a_piece_left_out_is_named_in_the_notice() {
        let said = wrote(
            &print(
                vec![pile("Delantero", 1)],
                vec![Skipped {
                    name: "Manga".to_owned(),
                    why: SheetError::Empty,
                }],
            ),
            "A4",
        );
        assert!(said.contains("sin imprimir: «Manga»"), "{said}");
    }

    /// A sheet that went out without its piece's own words says so here, which
    /// is the only door the owner uses: the command line said it already, and
    /// he prints from the button.
    #[test]
    fn the_labels_a_print_could_not_fit_are_counted_in_the_notice() {
        let mut held = pile("Delantero", 1);
        held.unsaid = 3;
        let said = wrote(&print(vec![held], vec![]), "A4");
        assert!(
            said.contains("3 rótulos de pieza sin sitio en su hoja"),
            "{said}"
        );
        // And a print that fitted every one of them says nothing about them.
        let clean = wrote(&print(vec![pile("Delantero", 1)], vec![]), "A4");
        assert!(!clean.contains("sin sitio"), "{clean}");
    }

    /// Two piles under one name cannot be separated on the table, and the
    /// notice is the only place that can say so: the sheets themselves say
    /// the name.
    #[test]
    fn two_pieces_under_one_name_are_named_in_the_notice() {
        let said = wrote(
            &print(vec![pile("Delantero", 1), pile("Delantero", 2)], vec![]),
            "A4",
        );
        assert!(
            said.contains("más de una pieza se llama «Delantero»"),
            "{said}"
        );
    }

    /// A product that reaches no paper names every piece to look at, which is
    /// what a person can act on with the drafting tab open.
    #[test]
    fn a_product_that_reaches_no_paper_names_every_piece_to_look_at() {
        let said = refused(&NothingPrinted {
            left_out: vec![Skipped {
                name: "Pieza 12".to_owned(),
                why: SheetError::Empty,
            }],
        });
        assert!(said.contains("«Pieza 12»"), "{said}");
    }

    /// The block the studio ships prints whole on either paper, which is the
    /// one case a person meets before they have drawn anything of their own.
    #[test]
    fn the_block_the_studio_ships_prints_every_piece_it_has() {
        let draft = Draft::from_doc(block::trousers()).expect("the block resolves");
        for paper in [Paper::A4, Paper::Carta] {
            let printed = export::to_pdf(&draft, paper.sheet()).expect("the block prints");
            assert!(printed.left_out.is_empty(), "{:?}", printed.left_out);
            assert_eq!(printed.pieces(), draft.doc().piece_keys().len());
        }
    }
}
