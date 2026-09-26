use toile_engine::draft::{Command, Doc, Draft, EdgeRange, PieceKey, PointKey, block};

/// Runs `toile doc`: a pattern, resolved and written out.
///
/// This is the headless door onto a pattern — the one a person reads over a
/// terminal and a language model reads over a pipe — so every number it prints
/// comes from the same resolution the viewer drapes. Without a path it reads
/// the base block the program carries, which is the file it ships.
pub fn run(args: &[String]) {
    let Some(mut doc) = asked_for(args) else {
        return;
    };
    if let Some(name) = flag(args, "--resolve-with") {
        let Some(key) = doc.mannequin_named(name) else {
            eprintln!("no hay ningún cuerpo llamado «{name}»");
            eprintln!("cuerpos: {}", bodies(&doc).join(", "));
            return;
        };
        if let Err(refused) = (Command::ResolveWith { mannequin: key }).apply(&mut doc) {
            eprintln!("no se pudo resolver con «{name}»: {refused}");
            return;
        }
    }
    match Draft::from_doc(doc) {
        Ok(draft) => print(&draft),
        Err(broken) => eprintln!("el documento no resuelve: {broken}"),
    }
}

/// The pattern the arguments name: a file, or the block carried in.
///
/// `None` once the reason it could not be read has been said, which is the
/// end of the run rather than a pattern to go on with.
fn asked_for(args: &[String]) -> Option<Doc> {
    let Some(path) = args.first().filter(|arg| !arg.starts_with("--")) else {
        return Some(block::trousers());
    };
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(why) => {
            eprintln!("no se pudo leer «{path}»: {why}");
            return None;
        }
    };
    match Doc::from_json(&text) {
        Ok(doc) => Some(doc),
        Err(why) => {
            eprintln!("«{path}» no es un patrón: {why}");
            None
        }
    }
}

/// The value written after `name`, when the arguments carry one.
fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    let at = args.iter().position(|arg| arg == name)?;
    args.get(at + 1).map(String::as_str)
}

/// The names of the bodies the document carries, in key order.
fn bodies(doc: &Doc) -> Vec<&str> {
    doc.mannequins
        .iter()
        .map(|(_, set)| set.name.as_str())
        .collect()
}

fn print(draft: &Draft) {
    let doc = draft.doc();
    let body = doc.measures().map_or("—", |set| set.name.as_str());
    println!("documento · resolver con «{body}»");
    println!("cuerpos: {}", bodies(doc).join(", "));

    println!("\nmedidas (cm)");
    if let Some(set) = doc.measures() {
        for (name, value) in &set.values {
            println!("  {name:<20}{value:>9.2}");
        }
    }

    println!("\nvariables (cm)");
    for (_, variable) in doc.variables.iter() {
        let value = draft.env().value(&variable.name);
        let resolved = value.map_or_else(|| "        —".to_owned(), |v| format!("{v:>9.2}"));
        println!(
            "  {:<20}{resolved}   = {}",
            variable.name,
            variable.value.source()
        );
    }

    println!("\nsujeción");
    let holds = holding(draft);
    if holds.is_empty() {
        println!("  nada: la prenda no se sujeta al cuerpo");
    }
    for line in holds {
        println!("  {line}");
    }

    for piece in doc.piece_keys() {
        println!();
        piece_report(draft, piece);
    }
}

/// What holds the garment on the body, one line each, in key order.
///
/// The two answers to one question, so they are read together: an elastic says
/// how hard a stretch is squeezed and a hang says which ring of the body it
/// belongs at. Neither is drawn on the paper, so without this the headless door
/// onto a pattern shows a waistband and a garment hung from the waist exactly
/// as it shows a plain hem.
fn holding(draft: &Draft) -> Vec<String> {
    let doc = draft.doc();
    let mut lines = Vec::new();
    for (_, elastic) in doc.elastics.iter() {
        lines.push(format!(
            "elástico  {} · {:.0} % · fuerza {}",
            stretch(draft, elastic.at),
            elastic.ratio * 100.0,
            elastic.strength
        ));
    }
    for (_, hang) in doc.hangs.iter() {
        lines.push(format!(
            "colgado   {} · de «{}»",
            stretch(draft, hang.at),
            hang.station
        ));
    }
    lines
}

/// A stretch of contour as a person reads it: the piece, and the two nodes it
/// runs between with the fraction of a tract when it does not end on one.
fn stretch(draft: &Draft, at: EdgeRange) -> String {
    let doc = draft.doc();
    let piece = at.head.piece;
    let name = doc.pieces.get(piece).map_or("—", |held| held.name.as_str());
    let end = |anchor: &toile_engine::draft::EdgeAnchor| {
        let node = name_of(draft, anchor.piece, anchor.from);
        if anchor.t == 0.0 {
            node
        } else {
            format!("{node}+{:.2}", anchor.t)
        }
    };
    format!("«{name}» {} → {}", end(&at.head), end(&at.tail))
}

/// One piece: its contour, its perimeter, its edge lengths and its defects.
fn piece_report(draft: &Draft, piece: PieceKey) {
    let doc = draft.doc();
    let Some(held) = doc.pieces.get(piece) else {
        return;
    };
    let nodes = draft.points_cm(piece);
    println!(
        "pieza «{}» · {} nodos · perímetro {:.2} cm · hilo {:.1}°",
        held.name,
        held.contour.len(),
        draft.perimeter_cm(piece),
        held.grain.radians().to_degrees()
    );
    println!(
        "  {:<3}{:<16}{:>9}{:>9}  {:<8}{:>9}",
        "#", "nodo", "x", "y", "tramo", "largo"
    );
    for (rank, &(point, [x, y])) in nodes.iter().enumerate() {
        let next = nodes[(rank + 1) % nodes.len()].0;
        println!(
            "  {:<3}{:<16}{x:>9.2}{y:>9.2}  {:<8}{:>9.2}",
            rank + 1,
            name_of(draft, piece, point),
            tract(draft, piece, rank),
            draft.run_length_cm(piece, point, next)
        );
    }
    let defects = draft.defects(piece);
    if defects.is_empty() {
        println!("  defectos: ninguno");
    } else {
        for defect in defects {
            println!("  defecto: {defect}");
        }
    }
}

/// What the piece calls one of its nodes.
fn name_of(draft: &Draft, piece: PieceKey, point: PointKey) -> String {
    draft
        .doc()
        .label_of(piece, point)
        .unwrap_or_else(|| format!("P{}", point.index()))
}

/// What runs from the node at `rank` to the next one.
fn tract(draft: &Draft, piece: PieceKey, rank: usize) -> &'static str {
    let segment = draft
        .doc()
        .pieces
        .get(piece)
        .and_then(|held| held.contour.get(rank))
        .map(|node| node.segment);
    match segment {
        Some(toile_engine::draft::Segment::Cubic { .. }) => "curva",
        _ => "recta",
    }
}

#[cfg(test)]
mod tests {
    use toile_engine::draft::{Elastic, Hang, Identity};

    use super::*;

    #[test]
    fn without_a_path_the_block_the_program_carries_is_read() {
        assert_eq!(asked_for(&[]), Some(block::trousers()));
    }

    #[test]
    fn a_flag_is_not_a_path() {
        let args = ["--resolve-with".to_owned(), "Talla 42".to_owned()];
        assert_eq!(asked_for(&args), Some(block::trousers()));
    }

    #[test]
    fn the_pattern_that_ships_is_read_from_its_file() {
        let shipped = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/pantalon-base.toile"
        );
        let args = [shipped.to_owned()];
        assert_eq!(asked_for(&args), Some(block::trousers()));
    }

    #[test]
    fn a_file_that_is_not_a_pattern_is_no_pattern_at_all() {
        let args = ["Cargo.toml".to_owned()];
        assert_eq!(asked_for(&args), None);
    }

    /// The headless door reads a garment held on the body and says what holds
    /// it, which is the whole of what a person outside the app can do with one
    /// today: no gesture puts a hang on, and this is where they see it is
    /// there.
    #[test]
    fn a_pattern_held_on_the_body_says_what_holds_it() {
        let mut doc = block::trousers();
        let front = doc.piece_named(block::FRONT).expect("the block draws one");
        let ends = ["cintura_cf", "cintura_lat"]
            .map(|label| doc.shows_label(front, label).expect("the block names it"));
        let at = EdgeRange::between(front, ends[0], ends[1]);
        Command::AddElastic {
            identity: Identity::New,
            elastic: Elastic::new(at, 0.85, 10.0),
        }
        .apply(&mut doc)
        .expect("both ends are nodes of the front");
        Command::AddHang {
            identity: Identity::New,
            hang: Hang::new(at, Hang::WAIST),
        }
        .apply(&mut doc)
        .expect("both ends are nodes of the front");

        // Through the file and not from the document in hand: what a person
        // outside the app has is bytes on disk, and the version those bytes
        // carry is the one this build has to read back.
        let written = doc.to_canonical_json();
        assert!(written.starts_with("{\n  \"toile\": 8,"), "{written}");
        let reread = Doc::from_json(&written).expect("this build reads it");
        let draft = Draft::from_doc(reread).expect("the block resolves");
        assert_eq!(
            holding(&draft),
            [
                "elástico  «Delantero» cintura_cf → cintura_lat · 85 % · fuerza 10",
                "colgado   «Delantero» cintura_cf → cintura_lat · de «cintura»",
            ]
        );
    }

    /// And a pattern nothing holds on says so, rather than showing an empty
    /// heading a reader has to guess at.
    #[test]
    fn a_pattern_nothing_holds_on_says_nothing_holds_it() {
        let draft = Draft::from_doc(block::trousers()).expect("the block resolves");
        assert!(holding(&draft).is_empty());
    }
}
