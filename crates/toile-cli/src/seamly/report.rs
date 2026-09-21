use std::fmt::Write as _;
use std::path::Path;

use toile_seamly::{Comment, Frozen, HelperKind, Product};

use super::check::{Check, PARITY};

/// The body and the variables.
mod body;
/// The spline lengths the file wrote, beside the curves'.
mod lengths;
/// The pieces, and what the product has no place for.
mod pieces;

/// Where the import read from and wrote to.
pub struct Files<'a> {
    pub source: &'a Path,
    pub product: &'a Path,
    pub svg: &'a Path,
    /// The person filed in the library, if the body became one.
    pub persona: Option<&'a Path>,
}

fn name(path: &Path) -> String {
    path.file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}

/// The import report, in Spanish, as Markdown: everything the product
/// carries and everything it does not, the pattern's comments included by
/// place only.
pub fn write(product: &Product, check: &Check, files: &Files<'_>, comments: &[Comment]) -> String {
    let mut out = String::new();
    let title = files
        .source
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    let _ = writeln!(out, "# {title} — importado a Toile\n");
    let _ = writeln!(
        out,
        "Origen: `{}` (Seamly2D). Producto: `{}`. Dibujo a escala real: `{}`.\n",
        name(files.source),
        name(files.product),
        name(files.svg)
    );
    if let Some(persona) = files.persona {
        let _ = writeln!(
            out,
            "Persona: `{}`, en la biblioteca de Toile.\n",
            name(persona)
        );
    }
    summary(&mut out, product, check);
    rules(&mut out);
    body::body(&mut out, product);
    body::variables(&mut out, product);
    frozen(&mut out, product, check);
    directions(&mut out, product);
    lengths::lengths(&mut out, product, check);
    pieces::pieces(&mut out, product);
    pieces::missing(&mut out, product, comments);
    out
}

fn summary(out: &mut String, product: &Product, check: &Check) {
    let report = &product.report;
    let curves: usize = report.pieces.iter().map(|p| p.curves.len()).sum();
    let corners: usize = report.pieces.iter().map(|p| p.nodes).sum();
    let _ = writeln!(out, "## Resumen\n");
    let _ = writeln!(
        out,
        "- {} piezas, {} puntos ({corners} esquinas y {} manijas de curva), {curves} curvas y {} piquetes.",
        report.pieces.len(),
        product.doc.points.len(),
        2 * curves,
        product.doc.notches.len()
    );
    let linked = if body::link(product).is_some() {
        ", copia vinculada de la persona de ese nombre en la biblioteca"
    } else {
        ""
    };
    let _ = writeln!(
        out,
        "- Un solo cuerpo, «{}»{linked}, con los valores del archivo de medidas que nombra el \
         patrón ({} medidas).",
        report.body,
        report.mapped.len() + report.carried.len()
    );
    let _ = writeln!(
        out,
        "- Resuelto en Toile, cada uno de los {} puntos cae a {:.1e} cm como mucho de donde \
         Seamly lo pone (se exige {PARITY:e} cm).",
        check.points, check.worst
    );
    let _ = writeln!(
        out,
        "- Con el cuerpo crecido ({}), {} puntos siguen al patrón exactamente (peor {:.1e} cm). \
         Los que dependen de algo congelado se apartan como dice la sección «Lo congelado».",
        grown(check),
        check.followed.0,
        check.followed.1
    );
    let stale = report.lengths.iter().filter(|n| lengths::stale(n)).count();
    if stale > 0 {
        let _ = writeln!(
            out,
            "- De las {} curvas que guardan su largo en el archivo, {stale} lo tienen viejo. \
             Toile mide cada curva y no lo lee; lo cuenta «Largos de curva guardados».",
            report.lengths.len()
        );
    }
    let (constructed, needed) = report.construction;
    let _ = writeln!(
        out,
        "- De los {constructed} puntos de construcción del patrón, el producto escribe {needed} \
         como fórmulas: los que son esquinas o sostienen una.\n"
    );
}

/// The grown body, as the measurements grown and their new values.
pub fn grown(check: &Check) -> String {
    let grown: Vec<String> = check
        .grown
        .iter()
        .map(|(n, v)| format!("{n} {v}"))
        .collect();
    grown.join(", ")
}

fn rules(out: &mut String) {
    let _ = writeln!(out, "## Cómo se tradujo\n");
    for line in [
        "Cada esquina y cada manija es un punto cuyas coordenadas son fórmulas sobre las medidas \
         y las variables: la construcción de Seamly desenrollada. Cambiar de cuerpo mueve el \
         producto como mueve el patrón.",
        "Un punto a distancia y ángulo recto de otro es la fórmula de ese otro más la distancia \
         en un eje. Un punto a lo largo de una línea avanza sobre el vector unitario de la línea, \
         y la longitud de una línea es la raíz de la suma de los cuadrados: sin trigonometría.",
        "Las constantes son los decimales del archivo, sumados sin redondear. Un ángulo que no \
         es recto se convierte en coseno y seno una sola vez, al importar.",
        "Las manijas de una curva conservan el ángulo y el largo que les da el archivo, desde el \
         punto del que salen, como en Seamly.",
        "Un arco de círculo es una o más cúbicas de no más de 90° cada una, con manijas de \
         4/3·tan(θ/4)·r. Una curva con las dos manijas en cero es la recta que traza.",
        "Cada curva se aplana en el menor número de muestras que la deja a menos de una décima \
         de milímetro de sí misma, como el bloque de pantalón de Toile.",
        "Las piezas van netas, sin margen de costura, recorridas en el orden de Seamly.",
    ] {
        let _ = writeln!(out, "- {line}");
    }
    let _ = writeln!(out);
}

fn frozen(out: &mut String, product: &Product, check: &Check) {
    let _ = writeln!(out, "## Lo congelado\n");
    let _ = writeln!(
        out,
        "Lo que ninguna fórmula puede seguir queda con el valor que tenía para este cuerpo. La \
         desviación es la que tiene con el cuerpo crecido de arriba.\n"
    );
    for note in &product.report.frozen {
        let drift = check
            .drift
            .get(&note.frozen)
            .map_or("—".to_owned(), |d| format!("{d:.4} cm"));
        match note.frozen {
            Frozen::SplineExcess(_) => {
                let _ = writeln!(
                    out,
                    "- **`{}`**: la curva es {:.6} cm más larga que su cuerda. La cuerda sigue al \
                     cuerpo; ese exceso quedó fijo, porque el largo de una curva no se escribe con \
                     sumas, multiplicaciones y raíces. Lo alcanzan {}. Desviación: {drift}.",
                    note.name,
                    note.value,
                    list(&note.reaches)
                );
            }
            Frozen::CutParameter(_) if note.reaches.is_empty() => {
                let _ = writeln!(
                    out,
                    "- **`{}`**: punto cortado sobre una curva, en t = {:.9}. No está en ninguna \
                     pieza —sólo en un trayecto interno, que no se importa—, así que el producto no \
                     lo lleva. Si entrara, su t quedaría fijo: seguiría sobre la curva, pero a otro \
                     largo del que pide su fórmula. Desviación que tendría: {drift}.",
                    note.name, note.value
                );
            }
            Frozen::CutParameter(_) => {
                let _ = writeln!(
                    out,
                    "- **`{}`**: punto cortado sobre una curva, con t fijo en {:.9}: sigue sobre la \
                     curva, pero a otro largo del que pide su fórmula. Lo alcanzan {}. \
                     Desviación: {drift}.",
                    note.name,
                    note.value,
                    list(&note.reaches)
                );
            }
        }
    }
    let _ = writeln!(out);
}

fn directions(out: &mut String, product: &Product) {
    let directions = &product.report.directions;
    if directions.is_empty() {
        return;
    }
    let _ = writeln!(out, "## Sentidos fijados\n");
    let _ = writeln!(
        out,
        "{} están a una distancia a lo largo de una línea horizontal o vertical cuyo sentido \
         depende de las fórmulas. Una fórmula exacta lo diría con un \
         valor absoluto; el producto toma el sentido que tiene para este cuerpo y escribe la \
         fórmula limpia. Sólo cambiaría si la línea se diera vuelta, y entonces la pieza de Seamly \
         también quedaría al revés.\n",
        list(directions)
    );
}

/// Names as a Spanish list of code spans.
pub fn list(names: &[String]) -> String {
    let spans: Vec<String> = names.iter().map(|n| format!("`{n}`")).collect();
    match spans.as_slice() {
        [] => "ninguno".to_owned(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} y {last}", rest.join(", ")),
    }
}

/// What an added variable stands for, in words.
pub fn stands_for(kind: &HelperKind) -> String {
    match kind {
        HelperKind::Drawn(name) => format!("`{name}`, citado en una fórmula del patrón"),
        HelperKind::Coordinate(point) => format!("una coordenada de `{point}`"),
    }
}
