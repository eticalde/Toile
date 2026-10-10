use std::fmt::Write as _;

use toile_engine::draft::Winding;
use toile_seamly::{Comment, PieceNote, Product};

/// The pieces as the product carries them.
pub fn pieces(out: &mut String, product: &Product) {
    let report = &product.report;
    let _ = writeln!(out, "## Piezas\n");
    let _ = writeln!(
        out,
        "Cada pieza de Seamly es una pieza del producto con el mismo nombre, recorrida en el orden \
         del patrón. Las manijas de una curva son puntos del producto como los demás, con nombre \
         `manija_…`.\n"
    );
    let _ = writeln!(
        out,
        "| pieza | esquinas | curvas | sentido | piquetes | líneas internas |\n\
         |---|---:|---:|---|---:|---:|"
    );
    for piece in &report.pieces {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} |",
            piece.name,
            piece.nodes,
            piece.curves.len(),
            winding(piece.winding),
            piece.notches.len(),
            piece.internal.len()
        );
    }
    let _ = writeln!(
        out,
        "\n| pieza | curva | muestras | se aparta (mm) |\n|---|---|---:|---:|"
    );
    for piece in &report.pieces {
        for (tract, samples, stray) in &piece.curves {
            let _ = writeln!(
                out,
                "| {} | {tract} | {samples} | {:.3} |",
                piece.name,
                stray * 10.0
            );
        }
    }
    let _ = writeln!(out);
}

fn winding(winding: Winding) -> &'static str {
    match winding {
        Winding::Cw => "horario",
        Winding::Ccw => "antihorario",
    }
}

/// What each piece says about being cut out, which the product keeps.
pub fn cut(out: &mut String, product: &Product) {
    let report = &product.report;
    let _ = writeln!(out, "## Cómo se corta cada pieza\n");
    let _ = writeln!(
        out,
        "La pieza del producto lleva su letra, cuántas se cortan, el margen de costura en \
         centímetros, el hilo y las líneas de su rótulo tal como están escritas en el patrón. El \
         contorno sigue siendo la línea de costura que dibuja Seamly: el papel dice el margen y \
         todavía nada desplaza la línea por él.\n"
    );
    let _ = writeln!(
        out,
        "| pieza | letra | cortar | margen (cm) | hilo | rótulo |\n|---|---|---:|---:|---|---|"
    );
    for piece in &report.pieces {
        let margin = piece
            .seam_allowance
            .map_or("neta".to_owned(), |width| format!("{width}"));
        let _ = writeln!(
            out,
            "| {} | {} | {} | {margin} | {} | {} |",
            piece.name,
            piece.letter,
            piece.quantity,
            grain(piece),
            piece.labels.join(" / ")
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "El rótulo va entero y sin leer: lo que dice el rótulo y el número que el archivo declara \
         salen los dos del mismo patrón, y el producto sostiene los dos sin elegir cuál cree.\n"
    );
    let written = report.pieces.iter().filter(|p| p.grain.is_some()).count();
    let _ = writeln!(
        out,
        "{}\n",
        if written == 0 {
            format!(
                "Ninguna de las {} piezas escribe un ángulo de hilo —su `<grainline>` va con la \
                 rotación vacía—, así que todas quedan con el hilo vertical de Toile. El ángulo \
                 que el archivo escriba, el producto lo guarda.",
                report.pieces.len()
            )
        } else {
            format!(
                "{written} de las {} piezas escriben el ángulo de su hilo y el producto lo guarda; \
                 las demás quedan con el hilo vertical de Toile.",
                report.pieces.len()
            )
        }
    );
}

/// The grain a piece is cut on, as the file writes it or as Toile leaves it.
fn grain(piece: &PieceNote) -> String {
    piece
        .grain
        .map_or("vertical".to_owned(), |degrees| format!("{degrees}°"))
}

/// What the pattern says that the product has no place for, or only half of.
pub fn missing(out: &mut String, product: &Product, comments: &[Comment]) {
    let report = &product.report;
    let _ = writeln!(out, "## Lo que Toile todavía no guarda\n");
    let _ = writeln!(
        out,
        "Nada de esto entra al producto, salvo donde el párrafo diga lo contrario. Queda anotado \
         aquí para que no se pierda.\n"
    );
    folds(out, &report.pieces);
    notches(out, &report.pieces);
    strokes(out, &report.pieces);
    let _ = writeln!(out, "### Hilo y posición\n");
    let _ = writeln!(
        out,
        "- Del `<grainline>` el producto se queda con el ángulo; el largo con que Seamly dibuja la \
         flecha y las puntas que le pone no, que son del dibujo y no del patrón."
    );
    let bias: Vec<String> = report
        .pieces
        .iter()
        .filter(|p| p.grain.is_none())
        .filter(|p| p.labels.iter().any(|l| l.to_lowercase().contains("bies")))
        .map(|p| p.name.clone())
        .collect();
    if !bias.is_empty() {
        let _ = writeln!(
            out,
            "- El rótulo de {} dice que se corta al bies y el archivo no le escribe ángulo: su \
             hilo hay que girarlo a mano.",
            super::list(&bias)
        );
    }
    let moved = report
        .pieces
        .iter()
        .any(|p| p.placement.iter().any(|v| v.abs() > 0.0));
    let _ = writeln!(
        out,
        "- {} La vista general de Toile pone en fila las piezas que nadie colocó; el SVG dibuja \
         cada pieza donde la construye el patrón.",
        if moved {
            "Las piezas que Seamly desplazó llevan ese desplazamiento como posición en la vista general."
        } else {
            "Ninguna pieza fue desplazada en Seamly (su `mx` y `my` son cero), así que ninguna lleva posición."
        }
    );
    let (constructed, needed) = report.construction;
    let _ = writeln!(
        out,
        "- {} de los {constructed} puntos de construcción no entran: son de trazos auxiliares que \
         ni una esquina ni una línea interna necesita.\n",
        constructed.saturating_sub(needed)
    );
    notes(out, comments);
}

/// Where the pattern carries comments, never what they say: they are its
/// author's, and this report is written into a folder that may be shared.
fn notes(out: &mut String, comments: &[Comment]) {
    let notes: Vec<&Comment> = comments.iter().filter(|c| !c.signature).collect();
    if notes.is_empty() {
        return;
    }
    let signed = if comments.iter().any(|c| c.signature) {
        ", sin contar la firma que Seamly pone en cada archivo"
    } else {
        ""
    };
    let _ = writeln!(out, "### Notas del patrón ({})\n", notes.len());
    let _ = writeln!(
        out,
        "El `.sm2d` lleva {} notas como comentarios XML{signed}. Toile no tiene dónde guardarlas, \
         así que no entran al producto, y este documento no las copia: siguen en el patrón original, \
         en estas líneas.\n",
        notes.len()
    );
    let _ = writeln!(out, "| dónde | notas | líneas |\n|---|---:|---|");
    let mut places: Vec<(Option<&str>, Vec<u32>)> = Vec::new();
    for note in notes {
        let block = note.block.as_deref();
        match places.iter_mut().find(|(at, _)| *at == block) {
            Some((_, lines)) => lines.push(note.line),
            None => places.push((block, vec![note.line])),
        }
    }
    for (block, lines) in places {
        let place = block.map_or("fuera de los bloques".to_owned(), |b| {
            format!("bloque «{b}»")
        });
        let lines: Vec<String> = lines.iter().map(u32::to_string).collect();
        let _ = writeln!(out, "| {place} | {} | {} |", lines.len(), lines.join(", "));
    }
    let _ = writeln!(out);
}

fn folds(out: &mut String, pieces: &[PieceNote]) {
    let folded: Vec<String> = pieces
        .iter()
        .filter(|p| p.on_fold)
        .map(|p| p.name.clone())
        .collect();
    if folded.is_empty() {
        return;
    }
    let _ = writeln!(out, "### Doblez\n");
    let _ = writeln!(
        out,
        "{} se corta al doblez según su rótulo. Toile tiene un eje de doblez, pero el patrón no \
         dice cuál es el borde doblado —en Seamly el doblez es sólo una marca del rótulo— y nada \
         en Toile despliega todavía una pieza doblada. Va la mitad, tal como está dibujada.\n",
        super::list(&folded)
    );
}

fn notches(out: &mut String, pieces: &[PieceNote]) {
    let any = pieces.iter().any(|p| !p.notches.is_empty());
    if !any {
        return;
    }
    let _ = writeln!(out, "### Piquetes\n");
    let _ = writeln!(
        out,
        "Toile guarda dónde está cada piquete y cuántos cortes tiene; la forma y la profundidad \
         no:\n"
    );
    for piece in pieces {
        for notch in &piece.notches {
            let _ = writeln!(
                out,
                "- {} en `{}`: `{}`, {} cm.",
                piece.name, notch.at, notch.kind, notch.length
            );
        }
    }
    let _ = writeln!(out);
}

/// What the product keeps of a line beyond where it runs and what it is for.
///
/// Nothing yet: Seamly's colour and weight say how a line is printed, which is
/// the drawing's business and not the pattern's, and the app strokes a line
/// from what it is for instead.
fn strokes(out: &mut String, pieces: &[PieceNote]) {
    let count: usize = pieces.iter().map(|p| p.internal.len()).sum();
    if count == 0 {
        return;
    }
    let _ = writeln!(out, "### El trazo de una línea interna\n");
    let _ = writeln!(
        out,
        "Los {count} trayectos internos entran al producto —los cuenta «Líneas internas»—, pero su \
         color y su grosor no: Toile dibuja una línea según para qué sirve, y de qué color se \
         imprime es cosa del dibujo, no del patrón.\n"
    );
}
