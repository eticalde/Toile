/// What the command says about the sheet it wrote.
mod said;

use std::path::{Path, PathBuf};

use toile_engine::draft::{Doc, Draft, PieceKey, block};
use toile_engine::export::{A4, CARTA, Paper, SheetError, to_pdf};

use crate::create;

/// Runs `toile pdf`: one piece of a pattern as a sheet to print at 1:1.
///
/// The headless door onto printing. A person reads the summary over a terminal
/// and finds the paper size and the piece in it, because a sheet printed at the
/// wrong scale is the one failure that only shows up after the cloth is cut.
pub fn run(args: &[String]) {
    match sheet(args) {
        Ok(lines) => lines.iter().for_each(|line| println!("{line}")),
        Err(why) => eprintln!("{why}"),
    }
}

/// What the command line asks for.
struct Asked {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    piece: Option<String>,
    paper: Paper,
}

/// Writes the sheet and says what went on it. Every refusal comes before the
/// one write.
fn sheet(args: &[String]) -> Result<Vec<String>, String> {
    let asked = parse(args)?;
    let doc = read(asked.input.as_deref())?;
    let draft = Draft::from_doc(doc).map_err(|why| format!("el documento no resuelve: {why}"))?;
    let piece = chosen(&draft, asked.piece.as_deref())?;
    let name = named(&draft, piece);
    let printed = to_pdf(&draft, piece, asked.paper).map_err(|why| refused(&name, &why))?;
    let path = beside(&asked, &name);
    if path.exists() {
        return Err(create::taken(&path));
    }
    create::file(&path, &printed.bytes)?;
    let mut said = vec![
        format!(
            "pieza «{name}» · {:.1} cm de perímetro · papel {} · escala 1:1",
            draft.perimeter_cm(piece),
            asked.paper.name
        ),
        said::sheets(&printed),
        said::drawn(&printed.inked, degrees(&draft, piece)),
        "imprime al 100 %, y mide el cuadrado de calibración de cada hoja con una regla antes de \
         cortar"
            .to_owned(),
    ];
    if printed.sheets > 1 {
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

/// Which way the piece's grain runs, in degrees.
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

/// The piece asked for by name, or the first one that resolves to a contour.
fn chosen(draft: &Draft, name: Option<&str>) -> Result<PieceKey, String> {
    let drawn: Vec<PieceKey> = draft
        .doc()
        .piece_keys()
        .into_iter()
        .filter(|&piece| draft.points_cm(piece).len() >= 3)
        .collect();
    let Some(name) = name else {
        return drawn
            .first()
            .copied()
            .ok_or_else(|| "ninguna pieza del patrón resuelve a un contorno".to_owned());
    };
    drawn
        .iter()
        .copied()
        .find(|&piece| named(draft, piece) == name)
        .ok_or_else(|| {
            let names: Vec<String> = drawn.iter().map(|&piece| named(draft, piece)).collect();
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

/// Where the sheet goes: where it was asked for, or beside the pattern under
/// the piece's own name, so ten pieces of one pattern do not land on each
/// other.
fn beside(asked: &Asked, name: &str) -> PathBuf {
    if let Some(output) = &asked.output {
        return output.clone();
    }
    let input = asked.input.as_deref().unwrap_or(Path::new("patron"));
    let stem = input
        .file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
    input.with_file_name(format!("{stem} - {name}.pdf"))
}

/// A refusal, in Spanish, for a person holding a pattern.
fn refused(name: &str, why: &SheetError) -> String {
    match why {
        SheetError::Empty => {
            format!("«{name}» no resuelve a ningún contorno: no hay nada que imprimir")
        }
        SheetError::TooMany {
            sheets,
            size_cm,
            paper,
        } => format!(
            "«{name}» mide {:.1} × {:.1} cm y saldría en {sheets} hojas de {paper}: eso es más de \
             una resma de papel, así que no se escribe",
            size_cm[0], size_cm[1]
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_a_path_the_block_the_program_carries_is_read() {
        assert_eq!(read(None), Ok(block::trousers()));
    }

    #[test]
    fn a_paper_is_named_in_either_language_and_in_any_case() {
        assert_eq!(paper("A4"), Ok(A4));
        assert_eq!(paper("Carta"), Ok(CARTA));
        assert_eq!(paper("letter"), Ok(CARTA));
        assert!(paper("a3").is_err());
    }

    #[test]
    fn the_sheet_lands_beside_the_pattern_under_the_name_of_its_piece() {
        let asked = Asked {
            input: Some(PathBuf::from("/tmp/Baggy Jeans.toile")),
            output: None,
            piece: None,
            paper: A4,
        };
        assert_eq!(
            beside(&asked, "Delantero"),
            PathBuf::from("/tmp/Baggy Jeans - Delantero.pdf")
        );
    }

    /// A piece bigger than a ream of paper is refused, and the refusal says
    /// both numbers a person needs: the size, which is what tells them
    /// which formula slipped, and the sheets, which is what tells them why
    /// nothing was written.
    #[test]
    fn a_piece_bigger_than_a_ream_is_refused_in_centimetres_and_in_sheets() {
        let why = SheetError::TooMany {
            sheets: 1_200,
            size_cm: [400.0, 1_040.0],
            paper: "A4",
        };
        let said = refused("Delantero", &why);
        assert!(said.contains("400.0 × 1040.0 cm"), "{said}");
        assert!(said.contains("1200 hojas"), "{said}");
    }
}
