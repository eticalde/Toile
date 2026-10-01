use toile_engine::draft::{Doc, Draft, EdgeAnchor, EdgeRange, PieceKey, PointKey, Segment};

/// Every seam the document declares, measured against its own tolerance.
mod seams;

/// The whole pattern, one line at a time: the body it resolves against, its
/// measurements and variables, what holds it on, and every piece.
pub fn pattern(draft: &Draft) -> Vec<String> {
    let doc = draft.doc();
    let body = doc.measures().map_or("—", |set| set.name.as_str());
    let mut lines = vec![
        format!("documento · resolver con «{body}»"),
        format!("cuerpos: {}", bodies(doc).join(", ")),
    ];
    lines.extend(measures(draft));
    lines.extend(variables(draft));
    lines.extend(sujecion(draft));
    lines.extend(seams::costuras(draft));
    for piece in doc.piece_keys() {
        lines.push(String::new());
        lines.extend(piece_lines(draft, piece));
    }
    lines
}

/// The names of the bodies the document carries, in key order.
pub fn bodies(doc: &Doc) -> Vec<&str> {
    doc.mannequins
        .iter()
        .map(|(_, set)| set.name.as_str())
        .collect()
}

/// The measurements of the body the pattern resolves against.
fn measures(draft: &Draft) -> Vec<String> {
    let mut lines = vec![String::new(), "medidas (cm)".to_owned()];
    if let Some(set) = draft.doc().measures() {
        lines.extend(
            set.values
                .iter()
                .map(|(name, value)| format!("  {name:<20}{value:>9.2}")),
        );
    }
    lines
}

/// Every variable, the number it resolves to, and the formula it is written as.
fn variables(draft: &Draft) -> Vec<String> {
    let mut lines = vec![String::new(), "variables (cm)".to_owned()];
    for (_, variable) in draft.doc().variables.iter() {
        let value = draft.env().value(&variable.name);
        let resolved = value.map_or_else(|| "        —".to_owned(), |v| format!("{v:>9.2}"));
        lines.push(format!(
            "  {:<20}{resolved}   = {}",
            variable.name,
            variable.value.source()
        ));
    }
    lines
}

/// What holds the garment on the body, under its own heading.
fn sujecion(draft: &Draft) -> Vec<String> {
    let mut lines = vec![String::new(), "sujeción".to_owned()];
    let holds = holding(draft);
    if holds.is_empty() {
        lines.push("  nada: la prenda no se sujeta al cuerpo".to_owned());
    }
    lines.extend(holds.into_iter().map(|line| format!("  {line}")));
    lines
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
    let end = |anchor: &EdgeAnchor| {
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
fn piece_lines(draft: &Draft, piece: PieceKey) -> Vec<String> {
    let doc = draft.doc();
    let Some(held) = doc.pieces.get(piece) else {
        return Vec::new();
    };
    let nodes = draft.points_cm(piece);
    let mut lines = vec![
        format!(
            "pieza «{}» · {} nodos · perímetro {:.2} cm · hilo {:.1}°",
            held.name,
            held.contour.len(),
            draft.perimeter_cm(piece),
            held.grain.radians().to_degrees()
        ),
        format!(
            "  {:<3}{:<16}{:>9}{:>9}  {:<8}{:>9}",
            "#", "nodo", "x", "y", "tramo", "largo"
        ),
    ];
    for (rank, &(point, [x, y])) in nodes.iter().enumerate() {
        let next = nodes[(rank + 1) % nodes.len()].0;
        lines.push(format!(
            "  {:<3}{:<16}{x:>9.2}{y:>9.2}  {:<8}{:>9.2}",
            rank + 1,
            name_of(draft, piece, point),
            tract(draft, piece, rank),
            draft.run_length_cm(piece, point, next)
        ));
    }
    let defects = draft.defects(piece);
    if defects.is_empty() {
        lines.push("  defectos: ninguno".to_owned());
    }
    lines.extend(defects.iter().map(|defect| format!("  defecto: {defect}")));
    lines
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
        Some(Segment::Cubic { .. }) => "curva",
        _ => "recta",
    }
}

#[cfg(test)]
mod tests;
