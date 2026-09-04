use toile_engine::draft::{Command, Doc, Draft, PieceKey, PointKey, block};

/// Runs `toile doc`: the base block, resolved and written out.
///
/// This is the headless door onto a pattern — the one a person reads over a
/// terminal and a language model reads over a pipe — so every number it prints
/// comes from the same resolution the viewer drapes.
pub fn run(args: &[String]) {
    let mut doc = block::trouser_front();
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

    for piece in doc.piece_keys() {
        println!();
        piece_report(draft, piece);
    }
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
