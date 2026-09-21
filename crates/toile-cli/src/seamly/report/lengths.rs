use std::fmt::Write as _;

use toile_seamly::{LengthNote, Product};

use super::{grown, list};
use crate::seamly::check::{Check, Trusting};

/// Whether the length the file wrote is no longer the curve's.
///
/// Compared at the decimals the table prints: a difference the table cannot
/// show is the file's own rounding, and one it shows is the curve having
/// moved since the number was written.
pub fn stale(note: &LengthNote) -> bool {
    format!("{:.4}", note.written) != format!("{:.4}", note.arc)
}

/// The spline lengths the file wrote that its curves have left behind, and
/// what that does to an evaluator that reads them.
pub fn lengths(out: &mut String, product: &Product, check: &Check) {
    let lengths = &product.report.lengths;
    if lengths.is_empty() {
        return;
    }
    let stale: Vec<&LengthNote> = lengths.iter().filter(|note| stale(note)).collect();
    let _ = writeln!(out, "## Largos de curva guardados\n");
    let _ = write!(
        out,
        "El `.sm2d` guarda en cada curva su largo (`length`), pero ese número es una copia, y \
         una copia se queda vieja cuando la curva se mueve. Toile no lo lee: mide la curva. "
    );
    if stale.is_empty() {
        let _ = writeln!(
            out,
            "Los {} largos que guarda este archivo coinciden con sus curvas.\n",
            lengths.len()
        );
        return;
    }
    let _ = writeln!(
        out,
        "En {} de las {} curvas que lo guardan, el número ya no es el largo de la curva:\n",
        stale.len(),
        lengths.len()
    );
    let _ = writeln!(
        out,
        "| curva | guardado (cm) | real (cm) | diferencia (cm) | lo lee |\n|---|---:|---:|---:|---|"
    );
    for note in &stale {
        let readers = if note.cited_by.is_empty() {
            "ninguna fórmula".to_owned()
        } else {
            list(&note.cited_by)
        };
        let _ = writeln!(
            out,
            "| `{}` | {} | {:.4} | {:+.4} | {readers} |",
            note.name,
            note.written,
            note.arc,
            note.arc - note.written
        );
    }
    let _ = writeln!(out);
    reader(out, &stale, check);
}

/// What the stale lengths mean for a script of one's own that reads the number
/// instead of measuring the curve.
fn reader(out: &mut String, stale: &[&LengthNote], check: &Check) {
    let read: Vec<String> = stale
        .iter()
        .filter(|note| !note.cited_by.is_empty())
        .map(|note| note.name.clone())
        .collect();
    let _ = writeln!(
        out,
        "### Para quien evalúe el patrón leyendo esos números\n"
    );
    if read.is_empty() {
        let _ = writeln!(
            out,
            "Ninguna fórmula lee esos largos, así que hoy no mueven ningún punto, tampoco en un \
             script propio que lea el número guardado del `.sm2d`. Lo harían en cuanto una \
             fórmula citara uno de esos nombres.\n"
        );
        return;
    }
    let _ = write!(
        out,
        "Un script propio que lea el número guardado del `.sm2d` en vez de medir la curva usa el \
         largo viejo de {} en las fórmulas que lo citan. ",
        list(&read)
    );
    match &check.trusting {
        Some(trusting) if !trusting.points.is_empty() => moved(out, trusting, check),
        Some(_) => {
            let _ = writeln!(out, "Aun así, ningún punto del producto depende de eso.");
        }
        None => {
            let _ = writeln!(
                out,
                "Leyendo los largos guardados, el patrón no se puede evaluar entero, así que no \
                 hay punto con el que comparar."
            );
        }
    }
    if read.len() < stale.len() {
        let _ = writeln!(
            out,
            "\nLas demás no las lee ninguna fórmula: hoy no mueven nada, y lo harían si una fórmula \
             citara su nombre."
        );
    }
    let _ = writeln!(
        out,
        "\nNada de esto alcanza al producto: Toile siempre mide la curva y nunca lee ese número.\n"
    );
}

fn moved(out: &mut String, trusting: &Trusting, check: &Check) {
    let _ = writeln!(
        out,
        "Con este cuerpo, eso pone {} esquinas del producto —{}— a hasta {:.1e} cm de donde las \
         pone Toile.",
        trusting.points.len(),
        list(&trusting.points),
        trusting.worst
    );
    if let Some(far) = trusting.grown {
        let _ = writeln!(
            out,
            "\nY el número guardado no sigue a la curva cuando cambia el cuerpo: con el cuerpo \
             crecido del resumen ({}), esas mismas esquinas se van hasta {far:.2} cm. El error \
             crece con lo que cambia la curva.",
            grown(check)
        );
    }
}
