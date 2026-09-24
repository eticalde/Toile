use std::fmt::Write as _;

use toile_doc::LineKind;
use toile_seamly::{Carried, InternalNote, PieceNote, Product, Refusal};

/// The internal lines of the product: what came over from the pattern's
/// internal paths, on which piece, and what did not.
pub fn lines(out: &mut String, product: &Product) {
    let pieces = &product.report.pieces;
    let notes: Vec<(&str, &InternalNote)> = pieces
        .iter()
        .flat_map(|piece| {
            piece
                .internal
                .iter()
                .map(move |note| (piece.name.as_str(), note))
        })
        .collect();
    if notes.is_empty() {
        return;
    }
    let count = notes.iter().filter(|(_, note)| drawn(note)).count();
    let _ = writeln!(out, "## Líneas internas ({count})\n");
    let _ = writeln!(
        out,
        "Un trayecto interno del patrón es una línea de la pieza que lo dibuja: un doblez, un \
         pespunte, la boca de un bolsillo, un ojal, una marca. Nada de esto se corta con el \
         contorno, y sin ello un patrón no se puede cortar en papel. De los {} trayectos del \
         patrón, {count} entraron al producto.\n",
        notes.len()
    );
    rules(out);
    table(out, &notes);
    per_piece(out, pieces);
    missing(out, &notes);
}

/// The one thing the file says about a path, and what the product reads from
/// it.
fn rules(out: &mut String) {
    for line in [
        "Seamly no guarda para qué sirve un trayecto: guarda su trazo, su nombre y una bandera \
         `cut`. La bandera se toma al pie de la letra —si dice que el corte abre la tela, la línea \
         es una ranura— y el trazo decide el resto: continuo, algo se apoya ahí (posición); \
         discontinuo, una línea con la que se razonó el trazado (referencia). Ninguna de esas dos \
         quita tela ni pide hilo, así que una suposición equivocada no estropea nada, y el tipo de \
         una línea se cambia de una vez en la mesa de patronaje.",
        "Un lugar de la línea que cae en una esquina de su propia pieza se guarda como lugar del \
         contorno: si la esquina se mueve, la línea se mueve con ella. Los demás son puntos \
         propios, con la construcción escrita como fórmulas, así que siguen al cuerpo como lo \
         sigue el patrón. Un lugar a media orilla no se ancla: su fracción a lo largo del tramo \
         sería un número que ninguna fórmula sigue, y un punto propio con fórmulas lo hace mejor.",
        "Dos lugares seguidos que caen en el mismo sitio con este cuerpo pero cuyas fórmulas no lo \
         demuestran se guardan los dos, unidos por un tramo de largo cero: es lo que dice la lista \
         de nodos del patrón, y una línea dibujada no paga nada por ello. Un contorno los rechaza, \
         porque ahí sí costaría un tramo.",
        "Las curvas se aplanan como las del contorno, en el menor número de muestras que las deja \
         a menos de una décima de milímetro de sí mismas.",
        "Un lugar que cuelga de algo que ninguna fórmula puede seguir —un punto cortado sobre una \
         curva, por ejemplo— aparece en «Lo congelado», con cuánto se desvía al cambiar el cuerpo.",
    ] {
        let _ = writeln!(out, "- {line}");
    }
    let _ = writeln!(out);
}

fn table(out: &mut String, notes: &[(&str, &InternalNote)]) {
    let _ = writeln!(
        out,
        "| pieza | línea | trazo | tipo | lugares | en el contorno | curvas | se aparta (mm) |\n\
         |---|---|---|---|---:|---:|---:|---:|"
    );
    for (piece, note) in notes {
        let stroke = stroke(&note.line_type);
        match note.carried {
            Carried::Line {
                kind,
                places,
                anchored,
                curves,
                stray,
            } => {
                let apart = if curves == 0 {
                    "—".to_owned()
                } else {
                    format!("{:.3}", stray * 10.0)
                };
                let _ = writeln!(
                    out,
                    "| {piece} | {} | {stroke} | {} | {places} | {anchored} | {curves} | {apart} |",
                    note.name,
                    kind_of(kind)
                );
            }
            Carried::Refused(_) => {
                let _ = writeln!(
                    out,
                    "| {piece} | {} | {stroke} | **no entró** | — | — | — | — |",
                    note.name
                );
            }
        }
    }
    let _ = writeln!(out);
}

/// How many lines each piece is drawn with, for the person laying out paper.
fn per_piece(out: &mut String, pieces: &[PieceNote]) {
    let _ = writeln!(out, "Por pieza, contando sólo las que entraron:\n");
    let _ = writeln!(
        out,
        "| pieza | líneas | lugares | curvas |\n|---|---:|---:|---:|"
    );
    for piece in pieces {
        let drawn: Vec<&InternalNote> = piece.internal.iter().filter(|note| drawn(note)).collect();
        if drawn.is_empty() {
            continue;
        }
        let sum = |pick: fn(&Carried) -> usize| -> usize {
            drawn.iter().map(|note| pick(&note.carried)).sum()
        };
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} |",
            piece.name,
            drawn.len(),
            sum(places),
            sum(curves)
        );
    }
    let _ = writeln!(out);
}

/// Every path the product drew nothing for, and why.
fn missing(out: &mut String, notes: &[(&str, &InternalNote)]) {
    let refused: Vec<(&str, &InternalNote, Refusal)> = notes
        .iter()
        .filter_map(|(piece, note)| match note.carried {
            Carried::Refused(why) => Some((*piece, *note, why)),
            Carried::Line { .. } => None,
        })
        .collect();
    if refused.is_empty() {
        let _ = writeln!(
            out,
            "No quedó ningún trayecto interno fuera del producto.\n"
        );
        return;
    }
    let _ = writeln!(out, "### Lo que no entró ({})\n", refused.len());
    for (piece, note, why) in refused {
        let _ = writeln!(out, "- {piece}, «{}»: {}.", note.name, because(why));
    }
    let _ = writeln!(out);
}

/// Why a path became no line, in words.
fn because(why: Refusal) -> &'static str {
    match why {
        Refusal::OnePlace => {
            "su recorrido llega a un solo lugar, y una línea va de un lugar a otro"
        }
    }
}

/// Whether the product drew a line for this path at all.
fn drawn(note: &InternalNote) -> bool {
    matches!(note.carried, Carried::Line { .. })
}

fn places(carried: &Carried) -> usize {
    match carried {
        Carried::Line { places, .. } => *places,
        Carried::Refused(_) => 0,
    }
}

fn curves(carried: &Carried) -> usize {
    match carried {
        Carried::Line { curves, .. } => *curves,
        Carried::Refused(_) => 0,
    }
}

/// A stroke as the file names it, in Spanish where Toile knows the word.
pub fn stroke(line_type: &str) -> &str {
    match line_type {
        "solidLine" => "continuo",
        "dashLine" => "discontinuo",
        other => other,
    }
}

/// What a line is for, in the word a person would use.
fn kind_of(kind: LineKind) -> &'static str {
    match kind {
        LineKind::Fold => "doblez",
        LineKind::Stitch => "pespunte",
        LineKind::Slit => "ranura",
        LineKind::Buttonhole => "ojal",
        LineKind::Placement => "posición",
        LineKind::Reference => "referencia",
    }
}
