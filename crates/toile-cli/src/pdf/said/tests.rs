use toile_engine::export::Laid;

use super::*;

/// A pile of one piece with nothing drawn on it, to say the paper with.
fn paper_of(name: &str, first_page: usize, sheets: usize, grid: [usize; 2]) -> Pile {
    shared(&[name], first_page, sheets, grid)
}

/// A pile several pieces share, each of which would have taken all of its
/// sheets on its own — which is what the strips of a waistband do.
fn shared(names: &[&str], first_page: usize, sheets: usize, grid: [usize; 2]) -> Pile {
    Pile {
        first_page,
        sheets,
        grid,
        pieces: names
            .iter()
            .map(|name| Laid {
                name: (*name).to_owned(),
                inked: Inked::default(),
                alone: sheets,
            })
            .collect(),
        unsaid: 0,
    }
}

/// A print of those piles, with no bytes: what is under test is the words.
fn print(piles: Vec<Pile>) -> Printed {
    Printed {
        bytes: Vec::new(),
        piles,
        left_out: Vec::new(),
    }
}

/// The one line the owner reads before he decides whether to print at all.
#[test]
fn the_header_counts_the_pieces_and_the_sheets_they_take() {
    let one = print(vec![paper_of("Cuadro", 1, 1, [1, 1])]);
    assert_eq!(
        paper(&one, "A4"),
        "producto · 1 pieza · 1 hoja de A4 · escala 1:1"
    );
    let two = print(vec![
        paper_of("Delantero", 1, 12, [2, 6]),
        paper_of("Trasero", 13, 18, [3, 6]),
    ]);
    assert_eq!(
        paper(&two, "Carta"),
        "producto · 2 piezas · 30 hojas de Carta · escala 1:1"
    );
}

/// A print that left a piece out says both counts, so the first line cannot be
/// read as the whole garment.
#[test]
fn a_print_missing_a_piece_says_how_many_of_how_many_it_has() {
    let mut short = print(vec![paper_of("Delantero", 1, 12, [2, 6])]);
    short.left_out.push(Skipped {
        name: "Manga".to_owned(),
        why: SheetError::Empty,
    });
    assert_eq!(
        paper(&short, "A4"),
        "producto · 1 de 2 piezas · 12 hojas de A4 · escala 1:1"
    );
}

/// A label with nowhere to go is counted on its pile's line and said again
/// under the piles, because it is the one thing a person cannot see on the
/// paper: the sheet looks finished, and the question it does not answer is the
/// one asked at the cutting table.
#[test]
fn a_label_with_no_room_on_its_sheet_is_counted_and_not_hushed() {
    let mut pile_of = paper_of("Delantero", 1, 12, [2, 6]);
    pile_of.unsaid = 2;
    let said = pile(&pile_of);
    assert!(said.contains("2 rótulos sin sitio en su hoja"), "{said}");
    let printed = print(vec![pile_of]);
    let aviso = unsaid(&printed).expect("two labels went unsaid");
    assert!(aviso.contains("2 rótulos de pieza sin sitio"), "{aviso}");
    assert!(aviso.contains("no el rótulo"), "{aviso}");
    // And a print that said everything says nothing about it.
    assert_eq!(unsaid(&print(vec![paper_of("Cuadro", 1, 1, [1, 1])])), None);
}

/// The pages of the file are what a print dialogue asks for, and the one thing
/// the printed sheet cannot say.
#[test]
fn a_pile_says_which_pages_of_the_file_it_is() {
    let said = pile(&paper_of("Trasero", 13, 18, [3, 6]));
    assert!(said.contains("págs. 13–30"), "{said}");
    assert!(
        said.contains("«Trasero» · 18 hojas · rejilla de 3 × 6"),
        "{said}"
    );
    let one = pile(&paper_of("Panel", 34, 1, [1, 1]));
    assert!(one.contains("pág. 34"), "{one}");
    assert!(one.contains("«Panel» · 1 hoja"), "{one}");
    assert!(!one.contains("rejilla"), "one sheet is no grid: {one}");
}

/// A pile several pieces share names all of them on its one line, because the
/// pages are the same pages and the pieces are not the same piece: three names
/// against one page range is how a person reads that the paper is shared.
#[test]
fn a_pile_several_pieces_share_names_every_one_of_them() {
    let said = pile(&shared(
        &["PRETINA", "TIRA PASACINTOS", "TIRA CADENA"],
        31,
        3,
        [3, 1],
    ));
    assert!(said.contains("págs. 31–33"), "{said}");
    assert!(
        said.contains("«PRETINA» · «TIRA PASACINTOS» · «TIRA CADENA» · 3 hojas"),
        "{said}"
    );
}

/// The number the owner reads before he buys paper: what the print cost, and
/// what it would have cost a piece to a pile.
#[test]
fn the_summary_says_what_sharing_the_paper_saved() {
    let strips = print(vec![shared(
        &["PRETINA", "TIRA PASACINTOS", "TIRA CADENA"],
        1,
        3,
        [3, 1],
    )]);
    let said = saved(&strips).expect("three piles became one");
    assert!(said.contains("ahorradas 6 hojas"), "{said}");
    assert!(said.contains("9 hojas si cada pieza fuera sola"), "{said}");
    let apart = print(vec![paper_of("Delantero", 1, 12, [2, 6])]);
    assert_eq!(saved(&apart), None, "nothing shared, nothing to say");
}

/// A grid with a hollow in it says so, because a person counting its cells by
/// hand gets a bigger number than the file has pages.
#[test]
fn a_pile_names_the_cells_that_carry_nothing() {
    let holed = pile(&paper_of("Ele", 1, 10, [3, 4]));
    assert!(holed.contains("menos 2 sin nada dibujado"), "{holed}");
}

/// The ink is counted in the words a person uses for it, and what there is none
/// of is not mentioned at all.
#[test]
fn the_summary_names_every_mark_that_reached_the_paper() {
    let full = Inked {
        lines: 3,
        notches: 1,
        darts: 2,
        names: 9,
    };
    assert_eq!(
        drawn(&full),
        "dibujado: 3 líneas internas · 1 piquete · 2 pinzas · 9 nombres de nodo · los contornos y \
         el hilo"
    );
    assert_eq!(
        drawn(&Inked::default()),
        "dibujado: sólo los contornos y el hilo"
    );
}

/// Two pieces under one name are two piles a person cannot separate, and the
/// summary is the only place that can say so: the sheets themselves say only
/// which piece they belong to.
#[test]
fn two_pieces_under_one_name_are_said_with_what_to_do_about_it() {
    let twins = print(vec![
        paper_of("Delantero", 1, 1, [1, 1]),
        paper_of("Delantero", 2, 1, [1, 1]),
    ]);
    let said = twinned(&twins);
    assert_eq!(said.len(), 1);
    assert!(said[0].contains("«Delantero»"), "{said:?}");
    assert!(said[0].contains("cámbiale el nombre"), "{said:?}");
    let apart = print(vec![
        paper_of("Delantero", 1, 1, [1, 1]),
        paper_of("Trasero", 2, 1, [1, 1]),
    ]);
    assert!(twinned(&apart).is_empty());
}

/// A piece left out is named on its own line, so that a person who asked for a
/// whole garment reads which panel is not in the file.
#[test]
fn a_piece_left_out_of_a_product_is_named_with_its_reason() {
    let said = left_out(&[Skipped {
        name: "Pieza 12".to_owned(),
        why: SheetError::Empty,
    }]);
    assert_eq!(
        said,
        ["sin imprimir: «Pieza 12» no resuelve a ningún contorno"]
    );
}

/// A piece bigger than a ream of paper is refused, and the refusal says both
/// numbers a person needs: the size, which is what tells them which formula
/// slipped, and the sheets, which is what tells them why nothing was written.
#[test]
fn a_piece_bigger_than_a_ream_is_refused_in_centimetres_and_in_sheets() {
    let said = refused(
        "Delantero",
        &SheetError::TooMany {
            sheets: 1_200,
            size_cm: [400.0, 1_040.0],
            paper: "A4",
        },
    );
    assert!(said.contains("400.0 × 1040.0 cm"), "{said}");
    assert!(said.contains("1200 hojas"), "{said}");
}

/// A product no piece of which reaches paper names every piece: one name would
/// have the person fix one piece and print nothing again.
#[test]
fn a_product_that_prints_nothing_names_every_piece_and_where_to_look() {
    let said = nothing(
        "Blusa Base",
        &NothingPrinted {
            left_out: vec![
                Skipped {
                    name: "Pieza 12".to_owned(),
                    why: SheetError::Empty,
                },
                Skipped {
                    name: "Pieza 13".to_owned(),
                    why: SheetError::Empty,
                },
            ],
        },
    );
    assert!(said.contains("«Blusa Base»"), "{said}");
    assert!(said.contains("«Pieza 12»"), "{said}");
    assert!(said.contains("«Pieza 13»"), "{said}");
    assert!(said.contains("toile doc"), "{said}");
}
