/// What the command says about the file it wrote.
mod said;

use std::path::{Path, PathBuf};

use toile_engine::draft::{Doc, Draft, PieceKey, block};
use toile_engine::export::{A4, CARTA, Paper, Printed, piece_to_pdf, to_pdf};

use crate::create;

/// What the pattern is called when no file was named, which is the block the
/// program carries.
const BLOCK: &str = "pantalón base";

/// Runs `toile pdf`: a pattern as sheets to print at 1:1.
///
/// The headless door onto printing. A person reads the summary over a terminal
/// and finds the paper size, the pieces and the pages each one takes, because a
/// sheet printed at the wrong scale is the one failure that only shows up after
/// the cloth is cut.
pub fn run(args: &[String]) {
    crate::report::said(sheets(args));
}

/// What the command line asks for.
struct Asked {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    piece: Option<String>,
    paper: Paper,
}

/// Writes the file and says what went on it. Every refusal comes before the one
/// write.
///
/// The whole product by default, because that is what a person prints: a
/// garment is one print job, one stack of paper and one thing to remember. One
/// piece is what `--pieza` is for, and it keeps its own file name so a re-cut
/// panel does not write over the garment.
fn sheets(args: &[String]) -> Result<Vec<String>, String> {
    let asked = parse(args)?;
    let doc = read(asked.input.as_deref())?;
    let draft = Draft::from_doc(doc).map_err(|why| format!("el documento no resuelve: {why}"))?;
    let (printed, mut said) = match asked.piece.clone() {
        Some(name) => alone(&draft, &name, asked.paper)?,
        None => whole(&draft, &asked, asked.paper)?,
    };
    let path = beside(&asked);
    if path.exists() {
        return Err(create::taken(&path));
    }
    create::file(&path, &printed.bytes)?;
    said.extend(said::left_out(&printed.left_out));
    said.push(
        "imprime al 100 %, y mide el cuadrado de calibración de cada hoja con una regla antes de \
         cortar"
            .to_owned(),
    );
    if printed.piles.iter().any(|pile| pile.sheets > 1) {
        said.push(
            "recorta cada hoja por la línea de puntos, solápala sobre su vecina y haz coincidir \
             las cruces"
                .to_owned(),
        );
    }
    said.push(format!(
        "escrito: {} ({} bytes)",
        path.display(),
        printed.bytes.len()
    ));
    Ok(said)
}

/// Every piece of the product, and the lines that say which pages are which.
fn whole(draft: &Draft, asked: &Asked, paper: Paper) -> Result<(Printed, Vec<String>), String> {
    let printed = to_pdf(draft, paper).map_err(|why| said::nothing(&pattern(asked), &why))?;
    let mut said = vec![said::paper(&printed, paper.name)];
    said.extend(printed.piles.iter().map(said::pile));
    said.push(said::drawn(&printed.inked()));
    said.extend(said::saved(&printed));
    said.extend(said::unsaid(&printed));
    said.extend(said::twinned(&printed));
    Ok((printed, said))
}

/// One piece of the product, for a person who re-cut one panel.
fn alone(draft: &Draft, name: &str, paper: Paper) -> Result<(Printed, Vec<String>), String> {
    let piece = chosen(draft, name)?;
    let name = named(draft, piece);
    let printed = piece_to_pdf(draft, piece, paper).map_err(|why| said::refused(&name, &why))?;
    let laid = &printed.piles[0];
    let said = vec![
        format!(
            "pieza «{name}» · {:.1} cm de perímetro · hilo a {:.1}° · papel {} · escala 1:1",
            draft.perimeter_cm(piece),
            degrees(draft, piece),
            paper.name
        ),
        said::pile(laid),
        said::drawn(&printed.inked()),
    ];
    Ok((printed, said))
}

/// Which way a piece's grain runs, in degrees.
fn degrees(draft: &Draft, piece: PieceKey) -> f64 {
    draft
        .doc()
        .pieces
        .get(piece)
        .map_or(0.0, |held| held.grain.radians().to_degrees())
}

/// The arguments, as a pattern, a destination, a piece and a paper.
fn parse(args: &[String]) -> Result<Asked, String> {
    let mut asked = Asked {
        input: None,
        output: None,
        piece: None,
        paper: A4,
    };
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--pieza" => asked.piece = Some(want(&mut rest, "--pieza")?),
            "--papel" => asked.paper = paper(&want(&mut rest, "--papel")?)?,
            flag if flag.starts_with("--") => {
                return Err(format!("no conozco la opción «{flag}»"));
            }
            path if asked.input.is_none() => asked.input = Some(PathBuf::from(path)),
            path if asked.output.is_none() => asked.output = Some(PathBuf::from(path)),
            extra => return Err(format!("no sé qué hacer con «{extra}»")),
        }
    }
    Ok(asked)
}

/// The value an option was given, or the reason it cannot go on without one.
fn want<'a>(rest: &mut impl Iterator<Item = &'a String>, flag: &str) -> Result<String, String> {
    rest.next()
        .cloned()
        .ok_or_else(|| format!("«{flag}» necesita un valor"))
}

/// The paper a name asks for.
fn paper(name: &str) -> Result<Paper, String> {
    match name.to_lowercase().as_str() {
        "a4" => Ok(A4),
        "carta" | "letter" => Ok(CARTA),
        other => Err(format!("no conozco el papel «{other}»: hay «a4» y «carta»")),
    }
}

/// The pattern the arguments name, or the block the program carries.
fn read(path: Option<&Path>) -> Result<Doc, String> {
    let Some(path) = path else {
        return Ok(block::trousers());
    };
    let text = std::fs::read_to_string(path)
        .map_err(|why| format!("no se pudo leer «{}»: {why}", path.display()))?;
    Doc::from_json(&text).map_err(|why| format!("«{}» no es un patrón: {why}", path.display()))
}

/// The piece asked for by name.
fn chosen(draft: &Draft, name: &str) -> Result<PieceKey, String> {
    let every = draft.doc().piece_keys();
    every
        .iter()
        .copied()
        .find(|&piece| named(draft, piece) == name)
        .ok_or_else(|| {
            let names: Vec<String> = every.iter().map(|&piece| named(draft, piece)).collect();
            format!(
                "no hay ninguna pieza llamada «{name}»\npiezas: {}",
                names.join(", ")
            )
        })
}

/// What the pattern calls a piece.
fn named(draft: &Draft, piece: PieceKey) -> String {
    draft
        .doc()
        .pieces
        .get(piece)
        .map_or_else(String::new, |held| held.name.clone())
}

/// What the pattern is called: its file's own name, else the block the program
/// carries when no file was named.
fn pattern(asked: &Asked) -> String {
    asked
        .input
        .as_deref()
        .and_then(Path::file_stem)
        .map_or_else(
            || BLOCK.to_owned(),
            |stem| stem.to_string_lossy().into_owned(),
        )
}

/// Where the file goes: where it was asked for, else beside the pattern — under
/// the pattern's own name for the whole product, and under the piece's as well
/// for one piece, so a re-cut panel never writes over the garment.
fn beside(asked: &Asked) -> PathBuf {
    if let Some(output) = &asked.output {
        return output.clone();
    }
    let stem = pattern(asked);
    let named = match &asked.piece {
        Some(piece) => format!("{stem} - {piece}.pdf"),
        None => format!("{stem}.pdf"),
    };
    match asked.input.as_deref() {
        Some(input) => input.with_file_name(named),
        None => PathBuf::from(named),
    }
}

#[cfg(test)]
mod tests;
