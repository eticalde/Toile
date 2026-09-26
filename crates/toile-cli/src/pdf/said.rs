use std::fmt::Write;

use toile_engine::export::{Inked, NothingPrinted, Pile, Printed, SheetError, Skipped};

/// What the whole print took, said the way a person buys paper.
///
/// The pieces come first because that is the number the owner compares against
/// his own count of what a garment is made of; the sheets are what he then has
/// to feed the printer. A print that left one out says both numbers, so the
/// first line is never a count a person could read as the whole garment.
pub fn paper(printed: &Printed, paper: &str) -> String {
    let laid = printed.pieces();
    let pieces = if printed.left_out.is_empty() {
        plural(laid, "pieza", "piezas")
    } else {
        format!("{laid} de {} piezas", laid + printed.left_out.len())
    };
    format!(
        "producto · {pieces} · {} de {paper} · escala 1:1",
        plural(printed.sheets(), "hoja", "hojas")
    )
}

/// One pile of sheets: which pages of the file it is, which pieces are on it,
/// and how much paper.
///
/// The pages and not only the count, because one file holds every pile and the
/// paper cannot say them: a sheet knows which sheet of its own pile it is, and
/// a print dialogue asks for the page of the file. This line is where the two
/// are put side by side.
///
/// Every piece of the pile is named, and a line naming two or three of them is
/// how a person reads that they share the paper — the sheets are the same
/// sheets and the pieces are not the same piece.
///
/// The blank cells are named and not quietly dropped: a person who counts the
/// cells of the grid by hand and gets a bigger number needs to read that the
/// difference is on purpose.
pub fn pile(pile: &Pile) -> String {
    let pages = if pile.sheets == 1 {
        format!("pág. {}", pile.first_page)
    } else {
        format!("págs. {}–{}", pile.first_page, pile.last_page())
    };
    let named: Vec<String> = pile
        .pieces
        .iter()
        .map(|piece| format!("«{}»", piece.name))
        .collect();
    let mut said = format!(
        "  {pages:<13}{} · {}",
        named.join(" · "),
        plural(pile.sheets, "hoja", "hojas")
    );
    if pile.sheets > 1 {
        let [cols, rows] = pile.grid;
        let _ = write!(said, " · rejilla de {cols} × {rows}");
    }
    if pile.blank() > 0 {
        let _ = write!(
            said,
            " menos {} sin nada dibujado, que no se imprimen",
            pile.blank()
        );
    }
    said
}

/// What sharing the paper saved, where anything was shared at all.
///
/// The number the owner is about to buy paper against. A belt-loop strap
/// forty-five centimetres long and under four tall takes three sheets of A4
/// with a plane to itself, and this line is how many of those went back in the
/// ream.
pub fn saved(printed: &Printed) -> Option<String> {
    let saved = printed.saved();
    (saved > 0).then(|| {
        format!(
            "ahorradas {} al compartir el papel: {} si cada pieza fuera sola",
            plural(saved, "hoja", "hojas"),
            plural(printed.alone(), "hoja", "hojas")
        )
    })
}

/// What reached the paper besides the lines the pieces are cut on.
///
/// Counted by the engine as it inked, not by this command reading the document:
/// a line one of whose places resolves nowhere is left off the sheet, and a
/// summary that counted the document would promise a mark the cutter will not
/// find.
pub fn drawn(inked: &Inked) -> String {
    let parts: Vec<String> = [
        counted(inked.lines, "línea interna", "líneas internas"),
        counted(inked.notches, "piquete", "piquetes"),
        counted(inked.names, "nombre de nodo", "nombres de nodo"),
    ]
    .into_iter()
    .flatten()
    .collect();
    if parts.is_empty() {
        return "dibujado: sólo los contornos y el hilo".to_owned();
    }
    format!("dibujado: {} · los contornos y el hilo", parts.join(" · "))
}

/// The pieces a print left on the table, one line each.
///
/// Said after the piles and not instead of them: the file was written, and what
/// a person has to know is which piece is not in it and why.
pub fn left_out(pieces: &[Skipped]) -> Vec<String> {
    pieces
        .iter()
        .map(|piece| format!("sin imprimir: «{}» {}", piece.name, why(&piece.why)))
        .collect()
}

/// The names two piles of the file share, when any do.
///
/// A person sorting the loose sheets into piles has only the piece each one
/// names to go by, so two pieces under one name are two piles that cannot be
/// told apart on the table. The file is written anyway: it is the name that has
/// to change, and it changes in the studio.
pub fn twinned(printed: &Printed) -> Vec<String> {
    printed
        .twinned()
        .iter()
        .map(|name| {
            format!(
                "aviso: más de una pieza se llama «{name}», así que sus hojas no se pueden \
                 separar; cámbiale el nombre a una"
            )
        })
        .collect()
}

/// Why one piece asked for by name could not be printed.
pub fn refused(name: &str, gone: &SheetError) -> String {
    format!("«{name}» {}, así que no se escribe nada", why(gone))
}

/// Why a whole product could not be printed: every piece of it, and what
/// stopped each one.
///
/// Sends the reader to `toile doc` for the defect itself. A contour that
/// crosses itself is a formula to fix, and that command already prints which
/// tract crosses which.
pub fn nothing(pattern: &str, gone: &NothingPrinted) -> String {
    let mut lines = vec![format!(
        "ninguna pieza de «{pattern}» llega al papel, así que no se escribe nada"
    )];
    lines.extend(
        gone.left_out
            .iter()
            .map(|piece| format!("  «{}» {}", piece.name, why(&piece.why))),
    );
    lines.push("mira «toile doc RUTA» para ver qué defecto tiene cada pieza".to_owned());
    lines.join("\n")
}

/// Why one piece did not reach paper, as a phrase that follows its own name.
fn why(gone: &SheetError) -> String {
    match gone {
        SheetError::Empty => "no resuelve a ningún contorno".to_owned(),
        SheetError::TooMany {
            sheets,
            size_cm,
            paper,
        } => format!(
            "mide {:.1} × {:.1} cm y saldría en {sheets} hojas de {paper}, más de una resma de \
             papel",
            size_cm[0], size_cm[1]
        ),
    }
}

/// A count with the word that goes with it.
fn plural(how_many: usize, one: &str, more: &str) -> String {
    if how_many == 1 {
        format!("1 {one}")
    } else {
        format!("{how_many} {more}")
    }
}

/// A count with the word that goes with it, or nothing at all when there is
/// none of that thing to say.
fn counted(how_many: usize, one: &str, more: &str) -> Option<String> {
    (how_many > 0).then(|| plural(how_many, one, more))
}

#[cfg(test)]
mod tests;
