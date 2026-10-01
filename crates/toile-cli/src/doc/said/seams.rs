use toile_engine::draft::{Draft, Lengths, Seam, SeamKind, SeamOrientation, tolerance_cm};

use super::stretch;

/// Every seam the document declares, with both sides measured and judged.
///
/// Under its own heading, and printed even when there are none: a pattern that
/// declares no seams drapes as a heap of loose panels, and that is the one
/// thing about a garment this door could not say. The alternative is a person
/// waiting out a whole drape to find out that nothing was ever going to close.
pub fn costuras(draft: &Draft) -> Vec<String> {
    let doc = draft.doc();
    let mut lines = vec![String::new(), "costuras".to_owned()];
    if doc.seams.is_empty() {
        lines.push("  ninguna: las piezas caen sueltas y no se cierra ninguna forma".to_owned());
        return lines;
    }
    for (rank, (_, seam)) in doc.seams.iter().enumerate() {
        lines.extend(one(draft, seam, rank + 1));
    }
    lines
}

/// One seam: what kind it is, what each of its two sides measures, and whether
/// the two agree by its own rule.
fn one(draft: &Draft, seam: &Seam, rank: usize) -> Vec<String> {
    let mut lines = vec![format!(
        "  costura {rank} · {} · {}",
        kind(seam),
        sense(seam.orientation)
    )];
    let Some(lengths) = Lengths::of(draft, seam) else {
        lines.push("    sin medir: un extremo no cae sobre un nodo de su pieza".to_owned());
        return lines;
    };
    for (side, cm) in [(&seam.a, lengths.a), (&seam.b, lengths.b)] {
        lines.push(format!("    {:<52}{cm:>8.2} cm", stretch(draft, *side)));
    }
    lines.push(format!("    {}", verdict(draft, seam, lengths)));
    lines
}

/// How far the two sides are from the length the seam asks of them, and what
/// the person holding the pattern can do about it.
///
/// The remedy and not just the number, because the number alone is a reading.
/// What it does not say, and must not, is that the garment will not close:
/// measured, two sides 2.11 cm apart are pulled shut to a fortieth of a
/// millimetre. What is wrong is the drawing against its own declared
/// tolerance, and that is worth knowing before cloth is cut rather than after
/// a drape.
fn verdict(draft: &Draft, seam: &Seam, lengths: Lengths) -> String {
    let out_by = lengths.excess(seam);
    let allowed = tolerance_cm(draft, seam);
    let head = format!(
        "difiere {:.2} cm · tolerancia {allowed:.2} cm",
        lengths.delta().abs()
    );
    match lengths.meets(draft, seam) {
        Some(true) => format!("{head} · dentro"),
        Some(false) => format!(
            "{head} · FUERA por {out_by:.2} cm: declárala embebida con esa \
             diferencia, o iguala los dos lados"
        ),
        None => format!("{head} · fruncida: el documento no dice qué lado se frunce"),
    }
}

/// What the seam does with a difference in length, in the words the glossary
/// uses for it.
fn kind(seam: &Seam) -> String {
    match seam.kind {
        SeamKind::Plain => "plana".to_owned(),
        SeamKind::Eased { expected_cm } => format!("embebida {expected_cm:.2} cm"),
        SeamKind::Gathered { ratio } => format!("fruncida {ratio:.2}×"),
    }
}

/// Which way the second side runs against the first.
fn sense(orientation: SeamOrientation) -> &'static str {
    match orientation {
        SeamOrientation::Aligned => "mismo sentido",
        SeamOrientation::Opposed => "sentidos opuestos",
    }
}

#[cfg(test)]
mod tests {
    use toile_engine::draft::{Doc, Draft, block};

    use super::costuras;

    /// The shipped block's two seams both measure and both pass, with the
    /// centimetres of each side on its own line.
    #[test]
    fn a_pattern_whose_sides_agree_says_so_seam_by_seam() {
        let draft = Draft::from_doc(block::trousers()).expect("the block resolves");
        let lines = costuras(&draft);
        assert_eq!(lines[1], "costuras");
        assert_eq!(
            lines.iter().filter(|l| l.starts_with("  costura ")).count(),
            2,
            "{lines:?}"
        );
        assert!(
            lines.iter().all(|l| !l.contains("FUERA")),
            "the block's sides agree: {lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.contains("tolerancia 0.50 cm · dentro")),
            "{lines:?}"
        );
    }

    /// A seam whose sides are further apart than its tolerance allows says so,
    /// in centimetres, and says what the person has to do about it.
    ///
    /// Through the block with its own tolerance tightened rather than a second
    /// pattern: what is being read is the verdict, and a fixture of this
    /// module's own would be a second truth about a seam.
    #[test]
    fn a_seam_outside_its_tolerance_says_what_it_would_take_to_close_it() {
        let mut doc = block::trousers();
        let key = doc.seams.keys().next().expect("the block declares a seam");
        doc.seams.get_mut(key).expect("the key is live").tolerance = Some(0.01);
        let draft = Draft::from_doc(doc).expect("the block resolves");
        let lines = costuras(&draft);
        let said = lines
            .iter()
            .find(|l| l.contains("FUERA"))
            .expect("the tightened seam is refused");
        assert!(said.contains("tolerancia 0.01 cm"), "{said}");
        assert!(said.contains("declárala embebida"), "{said}");
    }

    /// And a pattern that declares no seams says that, which is the one thing
    /// the owner's own jeans needed said: ten pieces and nothing joining them.
    #[test]
    fn a_pattern_with_no_seams_says_the_pieces_fall_loose() {
        let doc = Doc::new(
            block::trousers()
                .measures()
                .expect("the block names one")
                .clone(),
        );
        assert!(doc.seams.is_empty());
        let draft = Draft::from_doc(doc).expect("an empty pattern resolves");
        assert_eq!(
            costuras(&draft),
            [
                String::new(),
                "costuras".to_owned(),
                "  ninguna: las piezas caen sueltas y no se cierra ninguna forma".to_owned(),
            ]
        );
    }
}
