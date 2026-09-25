use toile_engine::export::{Inked, Printed};

/// How much paper the piece takes, said the way a person buys paper.
///
/// The blank cells are named and not quietly dropped: a person who counts the
/// sheets of a grid by hand and gets a bigger number needs to read that the
/// difference is on purpose.
pub fn sheets(printed: &Printed) -> String {
    if printed.sheets == 1 {
        return "1 hoja".to_owned();
    }
    let [cols, rows] = printed.grid;
    let blank = printed.blank();
    if blank == 0 {
        return format!("{} hojas · rejilla de {cols} × {rows}", printed.sheets);
    }
    format!(
        "{} hojas · rejilla de {cols} × {rows} menos {blank} sin nada dibujado, que no se imprimen",
        printed.sheets
    )
}

/// What reached the paper besides the line the piece is cut on.
///
/// Counted by the engine as it inked, not by this command reading the document:
/// a line one of whose places resolves nowhere is left off the sheet, and a
/// summary that counted the document would promise a mark the cutter will not
/// find.
pub fn drawn(inked: &Inked, degrees: f64) -> String {
    let parts: Vec<String> = [
        counted(inked.lines, "línea interna", "líneas internas"),
        counted(inked.notches, "piquete", "piquetes"),
        counted(inked.names, "nombre de nodo", "nombres de nodo"),
    ]
    .into_iter()
    .flatten()
    .collect();
    let grain = format!("hilo a {degrees:.1}°");
    if parts.is_empty() {
        return format!("dibujado: el contorno y el {grain}");
    }
    format!("dibujado: {} · {grain}", parts.join(" · "))
}

/// A count with the word that goes with it, or nothing at all when there is
/// none of that thing to say.
fn counted(how_many: usize, one: &str, more: &str) -> Option<String> {
    match how_many {
        0 => None,
        1 => Some(format!("1 {one}")),
        _ => Some(format!("{how_many} {more}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sheet count with nothing drawn on it, to say the paper with.
    fn paper(sheets: usize, grid: [usize; 2]) -> Printed {
        Printed {
            bytes: Vec::new(),
            sheets,
            grid,
            inked: Inked::default(),
        }
    }

    /// What the command says about the paper, which is the one line the owner
    /// reads before he decides whether to print at all.
    #[test]
    fn the_summary_counts_the_sheets_and_names_the_blank_ones() {
        assert_eq!(sheets(&paper(1, [1, 1])), "1 hoja");
        assert_eq!(sheets(&paper(12, [2, 6])), "12 hojas · rejilla de 2 × 6");
        let holed = sheets(&paper(10, [3, 4]));
        assert!(holed.contains("menos 2 sin nada dibujado"), "{holed}");
    }

    /// The ink is counted in the words a person uses for it, and what there is
    /// none of is not mentioned at all.
    #[test]
    fn the_summary_names_every_mark_that_reached_the_sheet() {
        let full = Inked {
            lines: 3,
            notches: 1,
            names: 9,
        };
        assert_eq!(
            drawn(&full, 90.0),
            "dibujado: 3 líneas internas · 1 piquete · 9 nombres de nodo · hilo a 90.0°"
        );
        assert_eq!(
            drawn(&Inked::default(), 0.0),
            "dibujado: el contorno y el hilo a 0.0°"
        );
        let one = Inked {
            lines: 1,
            notches: 0,
            names: 0,
        };
        assert_eq!(
            drawn(&one, 90.0),
            "dibujado: 1 línea interna · hilo a 90.0°"
        );
    }
}
