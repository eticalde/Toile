use super::*;

#[test]
fn without_a_path_the_block_the_program_carries_is_read() {
    assert_eq!(asked_for(&[]), Ok(block::trousers()));
}

#[test]
fn a_flag_is_not_a_path() {
    let args = ["--resolve-with".to_owned(), "Talla 42".to_owned()];
    assert_eq!(asked_for(&args), Ok(block::trousers()));
}

#[test]
fn the_pattern_that_ships_is_read_from_its_file() {
    let shipped = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/pantalon-base.toile"
    );
    let args = [shipped.to_owned()];
    assert_eq!(asked_for(&args), Ok(block::trousers()));
}

/// A refusal is a refusal all the way out, so that `$?` and what the command
/// said are the same answer: a script walking a folder of patterns cannot tell
/// a file it read from one that is not there if both exit zero.
#[test]
fn a_path_that_names_no_file_is_a_refusal_and_never_a_pattern() {
    let args = ["no/hay/tal/patrón.toile".to_owned()];
    let refused = printed(&args).expect_err("nothing to read");
    assert!(
        refused.starts_with("no se pudo leer «no/hay/tal"),
        "{refused}"
    );
}

#[test]
fn a_file_that_is_not_a_pattern_is_no_pattern_at_all() {
    let args = ["Cargo.toml".to_owned()];
    let refused = printed(&args).expect_err("not a pattern");
    assert!(
        refused.starts_with("«Cargo.toml» no es un patrón"),
        "{refused}"
    );
}

#[test]
fn a_body_the_document_does_not_carry_is_a_refusal_that_names_the_ones_it_does() {
    let args = ["--resolve-with".to_owned(), "Nadie".to_owned()];
    let refused = printed(&args).expect_err("no such body");
    assert!(
        refused.contains("no hay ningún cuerpo llamado «Nadie»"),
        "{refused}"
    );
    let carried = said::bodies(&block::trousers()).join(", ");
    assert!(refused.ends_with(&carried), "{refused}");
}

/// The block the program carries prints in full, which is what the split of
/// this command into a door and its lines must not have changed.
#[test]
fn the_block_the_program_carries_prints_its_body_its_sujecion_and_its_pieces() {
    let lines = printed(&[]).expect("the block resolves");
    assert!(
        lines[0].starts_with("documento · resolver con «"),
        "{lines:?}"
    );
    for heading in ["medidas (cm)", "variables (cm)", "sujeción"] {
        assert!(lines.iter().any(|line| line == heading), "{heading}");
    }
    let pieces = lines
        .iter()
        .filter(|line| line.starts_with("pieza «"))
        .count();
    assert_eq!(pieces, block::trousers().pieces.len());
}
