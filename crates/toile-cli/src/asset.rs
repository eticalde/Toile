use toile_engine::draft::block;

/// Where the base block is kept, from the root of the repository.
const ASSET: &str = "assets/pantalon-base.toile";

/// Runs `toile asset`: writes the base block where the program reads it from.
///
/// The file that ships is generated and never typed, so what a person opens
/// from the examples cannot drift from the block the tests draft.
pub fn run(args: &[String]) {
    let path = args.first().map_or(ASSET, String::as_str);
    let text = block::trousers().to_canonical_json();
    let said = std::fs::write(path, &text)
        .map(|()| vec![format!("{path}: {} bytes", text.len())])
        .map_err(|why| format!("no se pudo escribir «{path}»: {why}"));
    crate::report::said(said);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The file in the tree is the serialiser's own output. When this fails,
    /// the fix is `cargo run -p toile-cli -- asset`, never an edit by hand.
    #[test]
    fn the_asset_is_the_block_it_was_written_from() {
        let shipped = include_str!("../../../assets/pantalon-base.toile");
        assert_eq!(shipped, block::trousers().to_canonical_json());
    }
}
