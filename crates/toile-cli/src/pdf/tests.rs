use super::*;

/// What the command line would have asked for, with nothing set.
fn asked(input: Option<&str>, piece: Option<&str>) -> Asked {
    Asked {
        input: input.map(PathBuf::from),
        output: None,
        piece: piece.map(str::to_owned),
        paper: A4,
    }
}

#[test]
fn without_a_path_the_block_the_program_carries_is_read() {
    assert_eq!(read(None), Ok(block::trousers()));
}

#[test]
fn a_paper_is_named_in_either_language_and_in_any_case() {
    assert_eq!(paper("A4"), Ok(A4));
    assert_eq!(paper("Carta"), Ok(CARTA));
    assert_eq!(paper("letter"), Ok(CARTA));
    assert!(paper("a3").is_err());
}

/// The whole product lands under the pattern's own name, and one piece under
/// the pattern's and the piece's both — so printing one panel again never
/// writes over the garment, and printing the garment never writes over a panel.
#[test]
fn the_product_and_one_piece_of_it_land_on_different_files() {
    let whole = asked(Some("/tmp/Baggy Jeans.toile"), None);
    assert_eq!(beside(&whole), PathBuf::from("/tmp/Baggy Jeans.pdf"));
    let panel = asked(Some("/tmp/Baggy Jeans.toile"), Some("DELANTERO"));
    assert_eq!(
        beside(&panel),
        PathBuf::from("/tmp/Baggy Jeans - DELANTERO.pdf")
    );
}

/// A destination the person typed wins over both.
#[test]
fn a_destination_that_was_asked_for_is_where_the_file_goes() {
    let mut told = asked(Some("/tmp/Baggy Jeans.toile"), Some("PRETINA"));
    told.output = Some(PathBuf::from("/tmp/pretina.pdf"));
    assert_eq!(beside(&told), PathBuf::from("/tmp/pretina.pdf"));
}

/// A piece asked for by a name no piece carries is refused with the names there
/// are, which is the one thing that makes `--pieza` usable without the studio.
#[test]
fn a_name_no_piece_carries_is_refused_with_the_names_there_are() {
    let draft = Draft::from_doc(block::trousers()).expect("the block resolves");
    let refused = chosen(&draft, "Manga").expect_err("the block has no sleeve");
    assert!(refused.contains("«Manga»"), "{refused}");
    for piece in draft.doc().piece_keys() {
        assert!(refused.contains(&named(&draft, piece)), "{refused}");
    }
}
