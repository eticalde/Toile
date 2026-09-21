use std::fmt::Write as _;

use toile_doc::{Origin, PERSONA_EXTENSION};
use toile_seamly::Product;

use super::stands_for;

/// The link the product's body carries to a person in the library, if any.
pub fn link(product: &Product) -> Option<&Origin> {
    product.doc.measures()?.origin.as_ref()
}

/// The body: what was renamed to the catalogue, what kept its own name, and
/// what was left out.
pub fn body(out: &mut String, product: &Product) {
    let report = &product.report;
    let _ = writeln!(out, "## El cuerpo\n");
    let named = match link(product) {
        Some(origin) => format!(
            "copia de la persona «{}» de tu biblioteca de Toile (`{}.{PERSONA_EXTENSION}`, \
             medición del {})",
            report.body, origin.persona, origin.taken
        ),
        None => "con el nombre del archivo de medidas".to_owned(),
    };
    let _ = writeln!(
        out,
        "El producto lleva un único maniquí, «{}», {named}. Los datos personales del archivo de \
         medidas no se leen. Cada medida de Seamly toma el nombre del catálogo de Toile que le \
         corresponde según el archivo de datos `crates/toile-seamly/data/measurements.tsv`, que \
         sólo empareja medidas tomadas de la misma manera.\n",
        report.body
    );
    if link(product).is_some() {
        let _ = writeln!(
            out,
            "El producto guarda su propia copia de esas medidas y no depende de la biblioteca \
             para abrirse. Si más adelante cambian las medidas de «{0}» en la biblioteca, Toile lo \
             avisa al abrir el producto —«Las medidas de {0} difieren de tu biblioteca»— y ofrece \
             Actualizar o Mantener.\n",
            report.body
        );
    }
    let _ = writeln!(out, "| Seamly | Toile | cm |\n|---|---|---:|");
    for measure in report.mapped.iter().chain(&report.carried) {
        let toile = measure.toile.as_deref().unwrap_or("—");
        let _ = writeln!(
            out,
            "| `{}` | `{toile}` | {} |",
            measure.seamly, measure.value
        );
    }
    if !report.carried.is_empty() {
        let _ = writeln!(
            out,
            "\nSin nombre en el catálogo pero leídas por una fórmula, y por eso llevadas con su \
             propio nombre: {}.",
            super::list(
                &report
                    .carried
                    .iter()
                    .map(|m| m.seamly.clone())
                    .collect::<Vec<_>>()
            )
        );
    }
    if !report.left_out.is_empty() {
        let _ = writeln!(
            out,
            "\nFuera del maniquí, porque el catálogo no tiene una medida que se tome igual y \
             ninguna fórmula del patrón las lee:\n"
        );
        let _ = writeln!(out, "| Seamly | cm |\n|---|---:|");
        for measure in &report.left_out {
            let _ = writeln!(out, "| `{}` | {} |", measure.seamly, measure.value);
        }
    }
    let _ = writeln!(out);
}

/// The pattern's variables, then the ones the translation added.
pub fn variables(out: &mut String, product: &Product) {
    let report = &product.report;
    let _ = writeln!(out, "## Variables del patrón\n");
    let example = report.variables.first().map_or_else(String::new, |v| {
        format!(", así que `{}` se lee `{}`", v.seamly, v.toile)
    });
    let _ = writeln!(
        out,
        "Cada variable conserva su fórmula, con los nombres de Toile: sin `#` y en \
         minúsculas{example}. Una variable de Toile no tiene dónde guardar la descripción que \
         trae en Seamly, así que queda aquí.\n"
    );
    let _ = writeln!(
        out,
        "| Seamly | Toile | fórmula | descripción |\n|---|---|---|---|"
    );
    for note in &report.variables {
        let _ = writeln!(
            out,
            "| `{}` | `{}` | `{}` | {} |",
            note.seamly, note.toile, note.formula, note.description
        );
    }
    if report.helpers.is_empty() {
        let _ = writeln!(out);
        return;
    }
    let _ = writeln!(out, "\n## Variables auxiliares\n");
    let _ = writeln!(
        out,
        "Una fórmula de Toile lee medidas y variables, nunca otro punto. Dos tipos de variable \
         lo resuelven sin repetir fórmulas largas: cada `Line_` o `Spl_` que cita una fórmula del \
         patrón pasa a ser una variable de ese nombre, así la fórmula que la cita se lee como la \
         de Seamly; y un punto que necesita una raíz y sobre el que se construyen otros guarda sus \
         coordenadas en dos variables con su nombre.\n"
    );
    let _ = writeln!(out, "| variable | representa | fórmula |\n|---|---|---|");
    for helper in &report.helpers {
        let _ = writeln!(
            out,
            "| `{}` | {} | `{}` |",
            helper.name,
            stands_for(&helper.stands_for),
            helper.formula
        );
    }
    let _ = writeln!(out);
}
