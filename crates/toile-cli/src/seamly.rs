use std::path::{Path, PathBuf};

use toile_doc::{DocError, Origin, Persona, PersonaError};
use toile_engine::draft::Draft;
use toile_engine::export::{ExportError, to_svg};
use toile_seamly::{Measurements, Pattern, Product, import};

use self::args::{Asked, Wanted};
use crate::create;

/// What the command line asks for.
mod args;
/// Resolving the product in the engine and comparing it with the pattern.
mod check;
/// Filing a person in the user's library.
mod library;
/// The import report, as a person reads it.
mod report;
#[cfg(test)]
mod tests;

/// Runs `toile seamly`: a Seamly pattern in, a Toile product out, with the
/// report of what the translation did and the product drawn as an SVG.
///
/// The pattern names its measurement file relative to itself, and that is
/// the body the product resolves against, named after the file. Asked for a
/// person, the body is filed in the library under that name and the product
/// carries it as a linked copy. Nothing is written over an existing file: the
/// owner's folder and library are theirs.
pub fn run(args: &[String]) {
    crate::report::said(args::parse(args).and_then(|asked| migrate(&asked)));
}

/// Where the report and the drawing go, beside the product.
fn beside(output: &Path) -> (PathBuf, PathBuf) {
    let stem = output
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    let report = output.with_file_name(format!("{stem} - importado a Toile.md"));
    (report, output.with_extension("svg"))
}

/// Imports the pattern, checks the product, and writes it with its report and
/// drawing, and the person when asked; the lines to print when it all went
/// well. Every refusal comes before the first write.
fn migrate(asked: &Asked) -> Result<Vec<String>, String> {
    let (input, output) = (asked.input.as_path(), asked.output.as_path());
    let (report_path, svg_path) = beside(output);
    for path in [output, report_path.as_path(), svg_path.as_path()] {
        if path.exists() {
            return Err(create::taken(path));
        }
    }
    let wanted = asked
        .persona
        .as_ref()
        .map(|wanted| (wanted, Origin::stem_of(&wanted.name)));
    if let Some((wanted, stem)) = &wanted {
        let there = library::path(&wanted.library, stem).exists();
        match (wanted.existing, there) {
            (false, true) => return Err(library::taken(&wanted.library, stem)),
            (true, false) => return Err(library::missing(&wanted.library, stem)),
            _ => {}
        }
    }
    let (pattern, body, measured) = read(input)?;
    if pattern.blocks.iter().all(|block| block.pieces.is_empty()) {
        return Err(format!(
            "«{}» no tiene piezas: no hay nada que importar a Toile",
            input.display()
        ));
    }
    let mut product = import(&pattern, &body, &file_stem(&measured))
        .map_err(|why| format!("no se pudo importar: {why}"))?;
    let persona = match &wanted {
        Some((wanted, stem)) => Some(person(&mut product, wanted, stem, &measured)?),
        None => None,
    };
    let check = check::check(&product, &pattern, &body)
        .map_err(|why| format!("no se pudo comprobar el producto: {why}"))?;
    if !check.defects.is_empty() {
        return Err(format!(
            "el producto no resuelve limpio: {}",
            check.defects.join("; ")
        ));
    }
    let draft = Draft::from_doc(product.doc.clone())
        .map_err(|why| format!("el producto no resuelve: {why}"))?;
    let svg = to_svg(&draft).map_err(|ExportError::Empty| {
        "ninguna pieza del patrón resuelve a un contorno que dibujar".to_owned()
    })?;
    let filed = wanted
        .as_ref()
        .map(|(wanted, stem)| (library::path(&wanted.library, stem), !wanted.existing));
    let files = report::Files {
        source: input,
        product: output,
        svg: &svg_path,
        persona: filed.as_ref().map(|(at, new)| (at.as_path(), *new)),
    };
    let text = report::write(&product, &check, &files, &pattern.comments);
    let mut written = Vec::new();
    // First, so that a library folder that cannot be written leaves nothing
    // behind and a second run starts clean.
    if let (Some((wanted, stem)), Some((_, Some(json)))) = (&wanted, &persona) {
        written.push(library::file(&wanted.library, stem, json)?);
    }
    create::file(output, product.doc.to_canonical_json().as_bytes())?;
    create::file(&report_path, text.as_bytes())?;
    create::file(&svg_path, svg.as_bytes())?;
    written.extend([output.to_owned(), report_path, svg_path]);
    let person = persona
        .as_ref()
        .map(|(persona, json)| (persona, json.is_some()));
    Ok(summary(&product, &check, person, &written))
}

/// The pattern, its body, and the path of the body's file.
fn read(input: &Path) -> Result<(Pattern, Measurements, PathBuf), String> {
    let text = std::fs::read_to_string(input)
        .map_err(|why| format!("no se pudo leer «{}»: {why}", input.display()))?;
    let pattern = Pattern::parse(&text)
        .map_err(|why| format!("«{}» no es un patrón legible: {why}", input.display()))?;
    let relative = pattern
        .measurements
        .as_deref()
        .ok_or("el patrón no nombra un archivo de medidas")?;
    let path = input.parent().unwrap_or(Path::new(".")).join(relative);
    let text = std::fs::read_to_string(&path)
        .map_err(|why| format!("no se pudo leer «{}»: {why}", path.display()))?;
    let body = Measurements::parse(&text).map_err(|why| {
        format!(
            "«{}» no es un archivo de medidas legible: {why}",
            path.display()
        )
    })?;
    Ok((pattern, body, path))
}

/// The name a body takes from its file: the file's stem.
fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map_or_else(|| "cuerpo".to_owned(), |s| s.to_string_lossy().into_owned())
}

/// The person the product's body becomes a copy of, and the text to file her
/// under when this run is the one that files her.
///
/// Asked to link, nothing is built and nothing is written: the library's own
/// person is read and the product is linked to the measuring she already
/// carries. Her newest measuring has to be the day asked for, because that is
/// the one a copy takes — linking to her while naming another day would put a
/// date in the product that no measuring of hers answers to.
fn person(
    product: &mut Product,
    wanted: &Wanted,
    stem: &str,
    measured: &Path,
) -> Result<(Persona, Option<String>), String> {
    if wanted.existing {
        let persona = library::read(&wanted.library, stem)?;
        let taken = persona
            .current()
            .ok_or_else(|| format!("«{}» no tiene ninguna medición", wanted.name))?;
        if taken.date != wanted.taken {
            return Err(format!(
                "«{}» tiene como medición más reciente la del {}, no la del {}: el producto \
                 llevaría una copia de aquélla",
                wanted.name, taken.date, wanted.taken
            ));
        }
        linked(product, &persona, stem, wanted)?;
        return Ok((persona, None));
    }
    let source = measured
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let persona = product.persona(&wanted.name, &wanted.taken, &source);
    let json = persona.to_canonical_json().map_err(|why| match why {
        PersonaError::Invalid(DocError::NotADay(day)) => {
            format!("--tomada «{day}» no es una fecha escrita AAAA-MM-DD")
        }
        other => format!("la persona «{}» no se puede guardar: {other}", wanted.name),
    })?;
    linked(product, &persona, stem, wanted)?;
    Ok((persona, Some(json)))
}

/// Makes the product's body a linked copy of `persona`.
fn linked(
    product: &mut Product,
    persona: &Persona,
    stem: &str,
    wanted: &Wanted,
) -> Result<(), String> {
    product
        .link(persona, stem)
        .map_err(|why| format!("no se pudo vincular el producto a «{}»: {why}", wanted.name))
}

fn summary(
    product: &Product,
    check: &check::Check,
    persona: Option<(&Persona, bool)>,
    written: &[PathBuf],
) -> Vec<String> {
    let report = &product.report;
    let curves: usize = report.pieces.iter().map(|p| p.curves.len()).sum();
    let mut lines = vec![
        format!(
            "{} piezas · {} puntos · {} curvas · {} piquetes · {} líneas internas · formato {} · \
             cuerpo «{}»",
            report.pieces.len(),
            product.doc.points.len(),
            curves,
            product.doc.notches.len(),
            product.doc.lines.iter().count(),
            product.doc.format_version(),
            report.body
        ),
        format!(
            "paridad con el patrón: {} puntos, peor {:.1e} cm",
            check.points, check.worst
        ),
    ];
    for note in &report.frozen {
        if let Some(drift) = check.drift.get(&note.frozen) {
            lines.push(format!(
                "congelado «{}»: se desvía {drift:.4} cm con el cuerpo crecido",
                note.name
            ));
        }
    }
    if let Some((taken, filed)) = persona.and_then(|(who, filed)| Some((who.current()?, filed))) {
        let how = if filed {
            "archivada por esta importación"
        } else {
            "ya en la biblioteca, que no se toca"
        };
        lines.push(format!(
            "persona «{}», medición del {} ({how}): el producto lleva una copia vinculada",
            report.body, taken.date
        ));
    }
    lines.extend(
        written
            .iter()
            .map(|path| format!("escrito: {}", path.display())),
    );
    lines
}
