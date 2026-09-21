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
        "| pieza | esquinas | curvas | sentido | piquetes |\n|---|---:|---:|---|---:|"
    );
    for piece in &report.pieces {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} |",
            piece.name,
            piece.nodes,
            piece.curves.len(),
            winding(piece.winding),
            piece.notches.len()
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

/// Everything the pattern says that the product has no place for.
pub fn missing(out: &mut String, product: &Product, comments: &[Comment]) {
    let report = &product.report;
    let _ = writeln!(out, "## Lo que Toile todavía no guarda\n");
    let _ = writeln!(
        out,
        "Nada de esto entra al producto. Queda anotado aquí para que no se pierda.\n"
    );
    let _ = writeln!(out, "### Márgenes, cantidades y rótulos\n");
    let _ = writeln!(
        out,
        "El producto va neto: sin margen de costura, sin cantidad a cortar y sin rótulo.\n"
    );
    let _ = writeln!(
        out,
        "| pieza | letra | cortar | margen (cm) | al doblez | rótulo |\n|---|---|---:|---:|---|---|"
    );
    for piece in &report.pieces {
        let margin = piece
            .seam_allowance
            .map_or("—".to_owned(), |w| format!("{w}"));
        let fold = if piece.on_fold { "sí" } else { "no" };
        let _ = writeln!(
            out,
            "| {} | {} | {} | {margin} | {fold} | {} |",
            piece.name,
            piece.letter,
            piece.quantity,
            piece.labels.join(" / ")
        );
    }
    let _ = writeln!(out);
    folds(out, &report.pieces);
    notches(out, &report.pieces);
    internal(out, &report.pieces);
    let _ = writeln!(out, "### Hilo y posición\n");
    let _ = writeln!(
        out,
        "- Ninguna pieza del patrón tiene su línea de hilo visible ni un ángulo escrito, así que \
         todas quedan con el hilo vertical de Toile."
    );
    let bias: Vec<String> = report
        .pieces
        .iter()
        .filter(|p| p.labels.iter().any(|l| l.to_lowercase().contains("bies")))
        .map(|p| p.name.clone())
        .collect();
    if !bias.is_empty() {
        let _ = writeln!(
            out,
            "- El rótulo de {} dice que se corta al bies: su hilo hay que girarlo a mano.",
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
        "- {} de los {constructed} puntos de construcción no entran: son de trayectos internos o \
         de trazos auxiliares que ninguna esquina necesita.\n",
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

fn internal(out: &mut String, pieces: &[PieceNote]) {
    let count: usize = pieces.iter().map(|p| p.internal.len()).sum();
    if count == 0 {
        return;
    }
    let _ = writeln!(out, "### Trayectos internos ({count})\n");
    let _ = writeln!(
        out,
        "Toile todavía no tiene líneas internas: ranuras, pespuntes, dobleces y marcas se quedan \
         en el patrón de Seamly.\n"
    );
    let _ = writeln!(out, "| pieza | trayecto | trazo |\n|---|---|---|");
    for piece in pieces {
        for path in &piece.internal {
            let stroke = match path.line_type.as_str() {
                "solidLine" => "continuo",
                "dashLine" => "discontinuo",
                other => other,
            };
            let _ = writeln!(out, "| {} | {} | {stroke} |", piece.name, path.name);
        }
    }
    let _ = writeln!(out);
}
